// Used only by the std-only World (scene building / bounds).
#[cfg(feature = "std")]
use crate::bounds::BoundingBox;
use crate::csg::intersection_allowed;
use crate::intersections::Computations;
#[cfg(test)]
use crate::intersections::Intersection;
use crate::intersections::Intersections;
use crate::lights::*;
use crate::materials::lightning;
#[cfg(feature = "std")]
use crate::materials::Material;
use crate::matrices::transpose;
// Matrix the type is only named by std-side code (World tests/helpers); the
// no_std trace path uses inverse matrices by value without naming the type.
#[cfg(feature = "std")]
use crate::matrices::Matrix;
#[cfg(test)]
use crate::patterns::*;
use crate::rays::Ray;
use crate::shapes::*;
#[cfg(feature = "std")]
use crate::transformations::*;
use crate::tuples::*;

// Bounded scratch sizes for the iterative (recursion-free, GPU-compatible)
// traversal and shading. The traversal stack is height-bounded (it iterates a
// group's children via a cursor frame, not by pushing all of them), so this only
// needs to cover the deepest group/CSG nesting plus a small constant.
const MAX_TRAVERSAL_STACK: usize = 32;
// Longest parent chain a normal/point transform walks (scene hierarchy depth).
const MAX_TREE_DEPTH: usize = 32;
// Shading fans out to <= 2 rays (reflect + refract) per hit, depth-limited by
// `remaining` (default 5), so 2^(5+1) is a safe ceiling.
const MAX_SHADE_STACK: usize = 16;

const ZERO_RAY: Ray = Ray {
    origin: Point {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
    direction: Vector {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
};

// One frame of the explicit traversal stack. `tag` selects how the frame is
// interpreted; the spare fields carry whatever that state needs (a ray in node/
// group/csg-right frames, a child cursor for groups, a buffer start index for
// csg frames).
// u32, not u8: rust-gpu needs OpCapability Int8 for u8, which the default target
// doesn't enable.
const F_NODE: u32 = 0; // enter object `id` with `ray`; dispatch on kind
const F_GROUP: u32 = 1; // resume group `id` (local `ray`) at child `next`
const F_CSG_RIGHT: u32 = 2; // left done; enter csg `id`'s right child, then filter
const F_CSG_FILTER: u32 = 3; // filter the buffer region [`next`..end) for csg `id`

#[derive(Clone, Copy)]
struct Frame {
    tag: u32,
    id: usize,
    ray: Ray,
    next: usize,
}
impl Default for Frame {
    fn default() -> Self {
        Frame {
            tag: F_NODE,
            id: 0,
            ray: ZERO_RAY,
            next: 0,
        }
    }
}

// One pending shading ray in the iterative `color_at`: its contribution is
// `weight * surface_at(hit)`, and it may spawn weighted reflect/refract children.
#[derive(Clone, Copy)]
struct ShadeJob {
    ray: Ray,
    remaining: usize,
    weight: Color,
}
impl Default for ShadeJob {
    fn default() -> Self {
        ShadeJob {
            ray: ZERO_RAY,
            remaining: 0,
            weight: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
        }
    }
}

// The CPU host's scene container is std-only: it owns Vec arenas and runs scene
// building (groups/CSG/BVH). The GPU never builds scenes; it renders from
// uploaded buffers via `Scene`, which is no_std.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub objects: Vec<Primitive>,
    pub lights: Vec<Light>,
    // The logical children list of every object, indexed by object id (kept the
    // same length as `objects`). This is the host-side adjacency the build
    // helpers read and mutate; the heap-free flat projection the trace reads is
    // `child_indices`, kept current by `rebake`.
    pub children: Vec<Vec<usize>>,
    // The flat, contiguous projection of `children`: object `id`'s children are
    // `child_indices[objects[id].child_start .. + objects[id].child_count]`. The
    // trace reads this (never `children`), so a GPU buffer can be a plain slice.
    pub child_indices: Vec<usize>,
    // When true, a group culls its children against its bounding box before
    // recursing. Always correct to leave on; exposed only so a scene can render
    // the same world with it off to measure the speedup.
    pub use_bounds: bool,
}

// A borrowed, heap-free view of the parts of a `World` the ray trace and shading
// actually read: the flat object array, the lights, the flat child-index buffer
// and the bounds toggle. All the trace/shading methods live on `Scene` so they
// compile with no `Vec` and no `std` (the GPU shader will hand the same slices in
// from device buffers). `World` keeps thin forwarders that build a `Scene` and
// delegate, so existing host/test call sites are unchanged.
#[derive(Clone, Copy)]
pub struct Scene<'a> {
    pub objects: &'a [Primitive],
    pub lights: &'a [Light],
    pub child_indices: &'a [usize],
    pub use_bounds: bool,
}

#[cfg(feature = "std")]
impl World {
    pub fn new() -> Self {
        Self {
            objects: vec![],
            lights: vec![],
            children: vec![],
            child_indices: vec![],
            use_bounds: true,
        }
    }
    // Rebuild the flat `child_indices` projection from the logical `children`
    // side table and refresh every object's `child_start`/`child_count`. Called
    // at the end of every structural mutation so the trace (which reads only the
    // flat buffer) is always current without an explicit bake step.
    pub fn rebake(&mut self) {
        self.child_indices.clear();
        for id in 0..self.objects.len() {
            self.objects[id].child_start = self.child_indices.len() as u32;
            // `children` is kept the same length as `objects`, so this index is
            // always valid; guard anyway in case a primitive was pushed without
            // a matching children slot.
            if id < self.children.len() {
                self.child_indices.extend_from_slice(&self.children[id]);
                self.objects[id].child_count = self.children[id].len() as u32;
            } else {
                self.objects[id].child_count = 0;
            }
        }
    }
    // Build a borrowed `Scene` view over this world's slices. The trace/shading
    // methods live on `Scene`; the forwarders below call `self.scene().<same>()`.
    pub fn scene(&self) -> Scene {
        Scene {
            objects: &self.objects,
            lights: &self.lights,
            child_indices: &self.child_indices,
            use_bounds: self.use_bounds,
        }
    }
    pub fn intersect_world(&self, ray: &Ray) -> Intersections {
        self.scene().intersect_world(ray)
    }
    // Dispatch a ray to the arena object `id`. For a group, move the ray into
    // the group's space and recurse into its children. For a leaf, hand off to
    // the primitive's own `Primitive::intersect`, which applies the leaf's
    // transform. Transforms therefore compose down the hierarchy exactly as in
    // the book's Group::intersect.
    // Intersect the subtree rooted at `id` with `ray`, returning its hits (a
    // group's combined, a CSG's filtered). Iterative (no recursion) so it can run
    // on the GPU: an explicit stack of `Frame`s does a depth-first walk, moving
    // the ray into each group/CSG's space, culling against cached bounds, and
    // running the CSG surface filter as a post-order step over the buffer region
    // its subtree produced. The group cursor frame keeps the stack height-bounded
    // regardless of how many children a group has.
    // Intersect the subtree rooted at `id`. Forwards to the `Scene` impl (the
    // GPU-relevant trace lives there). The iterative `Frame` stack, bounds
    // culling and CSG post-order filter are all in `Scene::intersect_object`.
    pub fn intersect_object(&self, id: usize, ray: &Ray) -> Intersections {
        self.scene().intersect_object(id, ray)
    }
    pub fn includes(&self, node: usize, object: usize) -> bool {
        self.scene().includes(node, object)
    }
    pub fn filter_intersections(&self, csg_id: usize, xs: Intersections) -> Intersections {
        self.scene().filter_intersections(csg_id, xs)
    }
    // Object `id`'s bounding box in its own space, computed from scratch by
    // recursing into children (a leaf's `local_bounds`, a group/CSG's union of
    // its children's parent-space boxes). Unlike the cached `bounds`, this does
    // not depend on `compute_bounds` having run or on arena id ordering, so it is
    // safe to call mid-`divide` while the hierarchy is being rebuilt.
    fn object_bounds(&self, id: usize) -> BoundingBox {
        let obj = &self.objects[id];
        let children: Vec<usize> = match obj.kind {
            ShapeKind::Group => self.children[id].clone(),
            ShapeKind::Csg => obj.left().into_iter().chain(obj.right()).collect(),
            _ => return obj.local_bounds(),
        };
        let mut bb = BoundingBox::empty();
        for child in children {
            bb.add_box(&self.parent_space_bounds(child));
        }
        bb
    }
    // Object `id`'s box expressed in its parent's space: its own-space box lifted
    // through its transform.
    fn parent_space_bounds(&self, id: usize) -> BoundingBox {
        self.object_bounds(id)
            .transform(self.objects[id].get_transform())
    }
    // Compute and cache every group's and CSG node's bounding box. Call once after
    // a scene is fully assembled (and after any `divide`) and before rendering.
    // Recurses from each root so it is independent of arena id ordering, which
    // `divide` does not preserve when it reparents children into new sub-groups.
    pub fn compute_bounds(&mut self) {
        let roots: Vec<usize> = (0..self.objects.len())
            .filter(|&id| self.objects[id].parent().is_none())
            .collect();
        for root in roots {
            self.compute_bounds_of(root);
        }
        // Bounds do not change the adjacency, but rebake keeps the flat
        // projection consistent with any caller that mutated children first.
        self.rebake();
    }
    // Cache the box for `id` (if it is a group/CSG) and return its own-space box,
    // computing children first.
    fn compute_bounds_of(&mut self, id: usize) -> BoundingBox {
        let obj = &self.objects[id];
        let children: Vec<usize> = match obj.kind {
            ShapeKind::Group => self.children[id].clone(),
            ShapeKind::Csg => obj.left().into_iter().chain(obj.right()).collect(),
            _ => return obj.local_bounds(),
        };
        let mut bb = BoundingBox::empty();
        for child in children {
            let child_bounds = self.compute_bounds_of(child);
            let child_transform = self.objects[child].get_transform();
            bb.add_box(&child_bounds.transform(child_transform));
        }
        let obj = &mut self.objects[id];
        match obj.kind {
            ShapeKind::Group | ShapeKind::Csg => obj.set_bounds(bb),
            _ => {}
        }
        bb
    }
    // Split the children of group `id` by which half of the group's box they fall
    // entirely within. Children straddling the divide stay on the group; the
    // returned (left, right) lists are removed from it (to be re-homed by
    // `make_subgroup`). The book's `partition_children`.
    fn partition_children(&mut self, id: usize) -> (Vec<usize>, Vec<usize>) {
        let (left_box, right_box) = self.object_bounds(id).split();
        let children: Vec<usize> = if self.objects[id].kind == ShapeKind::Group {
            self.children[id].clone()
        } else {
            return (vec![], vec![]);
        };
        let mut left = vec![];
        let mut right = vec![];
        let mut kept = vec![];
        for child in children {
            let cbox = self.parent_space_bounds(child);
            if left_box.contains_box(&cbox) {
                left.push(child);
            } else if right_box.contains_box(&cbox) {
                right.push(child);
            } else {
                kept.push(child);
            }
        }
        if self.objects[id].kind == ShapeKind::Group {
            self.children[id] = kept;
        }
        (left, right)
    }
    // Wrap `children` in a new sub-group and attach it to group `id`. The book's
    // `make_subgroup`.
    fn make_subgroup(&mut self, id: usize, children: Vec<usize>) {
        let subgroup = self.objects.len();
        self.objects.push(Primitive::group());
        self.children.push(vec![]);
        self.objects[subgroup].set_parent(Some(id));
        for child in &children {
            self.objects[*child].set_parent(Some(subgroup));
        }
        if self.objects[subgroup].kind == ShapeKind::Group {
            self.children[subgroup] = children;
        }
        if self.objects[id].kind == ShapeKind::Group {
            self.children[id].push(subgroup);
        }
        self.rebake();
    }
    // Recursively subdivide group `id` into a bounding-volume hierarchy: when a
    // group has at least `threshold` children, partition them into two halves and
    // tuck each half into its own sub-group, then recurse. A CSG node has no
    // children to partition, so it just forwards `divide` to its two operands.
    // The book's `divide`. Run `compute_bounds` afterward to cache the new boxes.
    pub fn divide(&mut self, id: usize, threshold: usize) {
        match self.objects[id].kind {
            ShapeKind::Group => {
                if threshold <= self.children[id].len() {
                    let (left, right) = self.partition_children(id);
                    if !left.is_empty() {
                        self.make_subgroup(id, left);
                    }
                    if !right.is_empty() {
                        self.make_subgroup(id, right);
                    }
                }
                let children = self.children[id].clone();
                for child in children {
                    self.divide(child, threshold);
                }
            }
            ShapeKind::Csg => {
                let (left, right) = (self.objects[id].left(), self.objects[id].right());
                if let Some(left) = left {
                    self.divide(left, threshold);
                }
                if let Some(right) = right {
                    self.divide(right, threshold);
                }
            }
            _ => {}
        }
        // Keep the flat projection current after the restructuring (make_subgroup
        // already rebakes, but a divide that only recurses still ends consistent).
        self.rebake();
    }
    // The top-level ancestor of `id`: walk parent links until reaching a root.
    // Used by the interactive viewer to turn a picked leaf (which may be deep
    // inside a group/CSG) into the draggable object it belongs to.
    pub fn root_of(&self, mut id: usize) -> usize {
        while let Some(parent) = self.objects[id].parent() {
            id = parent;
        }
        id
    }
    // Whether object `id` is a sensible drag target: it must have finite bounds
    // (so not an infinite plane like a floor) and not be enormous (so not the
    // sky sphere). Everything else — a marble, the teapot group, the CSG widget —
    // is fair game.
    pub fn is_pickable(&self, id: usize) -> bool {
        // Bounds in the object's own frame (the leaf cube for a scaled sphere)
        // would miss its scale, so use the transformed bounds. For a root object
        // that is its world-space box.
        let b = self.parent_space_bounds(id);
        let finite = b.min.x.is_finite()
            && b.min.y.is_finite()
            && b.min.z.is_finite()
            && b.max.x.is_finite()
            && b.max.y.is_finite()
            && b.max.z.is_finite();
        if !finite {
            return false;
        }
        let extent = (b.max.x - b.min.x)
            .max(b.max.y - b.min.y)
            .max(b.max.z - b.min.z);
        extent < 100.0
    }
    // Book's world_to_object: walk up the parent chain applying each ancestor's
    // inverse transform, then this object's own, converting a world-space point
    // into the object's local space.
    pub fn world_to_object(&self, id: usize, point: Point) -> Point {
        self.scene().world_to_object(id, point)
    }
    pub fn normal_at(&self, id: usize, world_point: Point) -> Vector {
        self.scene().normal_at(id, world_point)
    }
    pub fn normal_at_uv(&self, id: usize, world_point: Point, u: Number, v: Number) -> Vector {
        self.scene().normal_at_uv(id, world_point, u, v)
    }
    // Append a top-level object and return its arena id.
    pub fn add_object(&mut self, object: Primitive) -> usize {
        let id = self.objects.len();
        self.objects.push(object);
        self.children.push(vec![]);
        self.rebake();
        id
    }
    // Append `child` and attach it to the group at `group_id`: set the child's
    // parent and record its id in the group's children. Mirrors the book's
    // Group::add_child.
    pub fn add_child(&mut self, group_id: usize, mut child: Primitive) -> usize {
        child.set_parent(Some(group_id));
        let id = self.objects.len();
        self.objects.push(child);
        self.children.push(vec![]);
        if self.objects[group_id].kind == ShapeKind::Group {
            self.children[group_id].push(id);
        }
        self.rebake();
        id
    }
    // Attach the two children of a CSG node. Each must already be in the arena
    // (added after the CSG node, so its id is higher, which keeps the reverse-id
    // ordering `compute_bounds` depends on). The book's `csg(op, left, right)`
    // sets the children's parent to the CSG; this does the same by id.
    pub fn set_csg_children(&mut self, csg_id: usize, left: usize, right: usize) {
        self.objects[left].set_parent(Some(csg_id));
        self.objects[right].set_parent(Some(csg_id));
        if self.objects[csg_id].kind == ShapeKind::Csg {
            self.objects[csg_id].set_left(left);
            self.objects[csg_id].set_right(right);
        }
        self.rebake();
    }
    // The direct (local) surface color at a hit: the Phong contribution of every
    // light, shadow-tested independently, with no reflection/refraction. Shared
    // by `shade_hit` and the iterative `color_at` so the two stay in lockstep.
    pub fn shade_hit(&self, comps: Computations, remaining: usize) -> Color {
        self.scene().shade_hit(comps, remaining)
    }
    pub fn color_at(&self, ray: &Ray, remaining: usize) -> Color {
        self.scene().color_at(ray, remaining)
    }
    pub fn is_shadowed(&self, point: Point, light: &Light) -> bool {
        self.scene().is_shadowed(point, light)
    }
    pub fn is_shadowed_at(&self, light_position: Point, point: Point) -> bool {
        self.scene().is_shadowed_at(light_position, point)
    }
    pub fn intensity_at(&self, point: Point, light: &Light) -> Number {
        self.scene().intensity_at(point, light)
    }
    pub fn reflected_color(&self, comps: &Computations, remaining: usize) -> Color {
        self.scene().reflected_color(comps, remaining)
    }
    pub fn refracted_color(&self, comps: &Computations, remaining: usize) -> Color {
        self.scene().refracted_color(comps, remaining)
    }
}

// The actual ray trace and shading, on the borrowed `Scene` view. These are the
// methods the GPU shader will run; they touch only `objects`/`lights`/
// `child_indices`/`use_bounds` slices, so they compile with no `Vec` and no
// `std`. The `World` methods above forward here.
impl<'a> Scene<'a> {
    pub fn intersect_world(&self, ray: &Ray) -> Intersections {
        let mut intersections = Intersections::empty();
        // Only roots are traversed here; children are reached by intersect_object,
        // so a child must not be intersected a second time. Index loop (not
        // .iter().enumerate()) so rust-gpu can lower it: SPIR-V has no slice
        // iterators / pointer arithmetic.
        let mut id = 0;
        while id < self.objects.len() {
            if self.objects[id].parent().is_none() {
                let sub = self.intersect_object(id, ray);
                intersections.extend(&sub);
            }
            id += 1;
        }
        // Sort once, here, now that every root has contributed. `color_at` and the
        // tests rely on `intersect_world` returning hits in t-order.
        intersections.sort();
        intersections
    }
    pub fn intersect_object(&self, id: usize, ray: &Ray) -> Intersections {
        let mut out = Intersections::empty();
        let mut stack = [Frame::default(); MAX_TRAVERSAL_STACK];
        let mut sp = 0usize;
        stack[sp] = Frame {
            tag: F_NODE,
            id,
            ray: *ray,
            next: 0,
        };
        sp += 1;

        while sp > 0 {
            sp -= 1;
            let f = stack[sp];
            match f.tag {
                F_NODE => {
                    let object = &self.objects[f.id];
                    match object.kind {
                        ShapeKind::Group => {
                            let local_ray = f.ray.transform(object.get_inverse_transform());
                            // Read the bounds fields directly (not Option<BoundingBox>,
                            // which rust-gpu can't lower).
                            if self.use_bounds && object.has_bounds != 0 {
                                if !object.bounds.intersects(&local_ray) {
                                    continue;
                                }
                            }
                            stack[sp] = Frame {
                                tag: F_GROUP,
                                id: f.id,
                                ray: local_ray,
                                next: 0,
                            };
                            sp += 1;
                        }
                        ShapeKind::Csg => {
                            let local_ray = f.ray.transform(object.get_inverse_transform());
                            // Read the bounds fields directly (not Option<BoundingBox>,
                            // which rust-gpu can't lower).
                            if self.use_bounds && object.has_bounds != 0 {
                                if !object.bounds.intersects(&local_ray) {
                                    continue;
                                }
                            }
                            let start = out.len;
                            stack[sp] = Frame {
                                tag: F_CSG_RIGHT,
                                id: f.id,
                                ray: local_ray,
                                next: start,
                            };
                            sp += 1;
                            stack[sp] = Frame {
                                tag: F_NODE,
                                id: object.left().unwrap(),
                                ray: local_ray,
                                next: 0,
                            };
                            sp += 1;
                        }
                        _ => object.intersect_into(&f.ray, f.id, &mut out),
                    }
                }
                F_GROUP => {
                    let object = &self.objects[f.id];
                    // Children come from the flat buffer, not a per-primitive Vec.
                    let start = object.child_start as usize;
                    let count = object.child_count as usize;
                    if f.next < count {
                        let child = self.child_indices[start + f.next];
                        stack[sp] = Frame {
                            tag: F_GROUP,
                            id: f.id,
                            ray: f.ray,
                            next: f.next + 1,
                        };
                        sp += 1;
                        stack[sp] = Frame {
                            tag: F_NODE,
                            id: child,
                            ray: f.ray,
                            next: 0,
                        };
                        sp += 1;
                    }
                }
                F_CSG_RIGHT => {
                    let object = &self.objects[f.id];
                    stack[sp] = Frame {
                        tag: F_CSG_FILTER,
                        id: f.id,
                        ray: f.ray,
                        next: f.next,
                    };
                    sp += 1;
                    stack[sp] = Frame {
                        tag: F_NODE,
                        id: object.right().unwrap(),
                        ray: f.ray,
                        next: 0,
                    };
                    sp += 1;
                }
                _ => self.filter_region(f.id, &mut out, f.next),
            }
            debug_assert!(sp <= MAX_TRAVERSAL_STACK, "traversal stack overflow");
        }
        out
    }
    fn filter_region(&self, csg_id: usize, out: &mut Intersections, start: usize) {
        let csg = &self.objects[csg_id];
        let (operation, left) = match csg.kind {
            ShapeKind::Csg => (csg.operation, csg.left().unwrap()),
            _ => return,
        };
        let end = out.len;
        let mut i = start + 1;
        while i < end {
            let key = out.xs[i];
            let mut j = i;
            while j > start && out.xs[j - 1].t > key.t {
                out.xs[j] = out.xs[j - 1];
                j -= 1;
            }
            out.xs[j] = key;
            i += 1;
        }
        let mut inside_left = false;
        let mut inside_right = false;
        let mut w = start;
        let mut k = start;
        while k < end {
            let inter = out.xs[k];
            let left_hit = self.includes(left, inter.object_id);
            if intersection_allowed(operation, left_hit, inside_left, inside_right) {
                out.xs[w] = inter;
                w += 1;
            }
            if left_hit {
                inside_left = !inside_left;
            } else {
                inside_right = !inside_right;
            }
            k += 1;
        }
        out.len = w;
    }
    pub fn includes(&self, node: usize, object: usize) -> bool {
        let mut cur = object;
        loop {
            if cur == node {
                return true;
            }
            match self.objects[cur].parent() {
                Some(p) => cur = p,
                None => return false,
            }
        }
    }
    pub fn filter_intersections(&self, csg_id: usize, mut xs: Intersections) -> Intersections {
        let csg = &self.objects[csg_id];
        let (operation, left) = match csg.kind {
            ShapeKind::Csg => (csg.operation, csg.left().unwrap()),
            _ => return xs,
        };
        xs.sort();
        let mut inside_left = false;
        let mut inside_right = false;
        let mut result = Intersections::empty();
        for idx in 0..xs.len {
            let intersection = xs.xs[idx];
            let left_hit = self.includes(left, intersection.object_id);
            if intersection_allowed(operation, left_hit, inside_left, inside_right) {
                result.push(intersection);
            }
            if left_hit {
                inside_left = !inside_left;
            } else {
                inside_right = !inside_right;
            }
        }
        result
    }
    pub fn world_to_object(&self, id: usize, point: Point) -> Point {
        let mut chain = [0usize; MAX_TREE_DEPTH];
        let mut n = 0;
        let mut cur = id;
        loop {
            chain[n] = cur;
            n += 1;
            match self.objects[cur].parent() {
                Some(parent) if n < MAX_TREE_DEPTH => cur = parent,
                _ => break,
            }
        }
        let mut p = point;
        let mut k = n;
        while k > 0 {
            k -= 1;
            let inverse = self.objects[chain[k]].get_inverse_transform();
            p = inverse * p;
        }
        p
    }
    fn normal_to_world(&self, id: usize, normal: Vector) -> Vector {
        let mut normal = normal;
        let mut cur = id;
        loop {
            let inverse = self.objects[cur].get_inverse_transform();
            normal = (transpose(&inverse) * normal).normalize();
            match self.objects[cur].parent() {
                Some(parent) => cur = parent,
                None => break,
            }
        }
        normal
    }
    pub fn normal_at(&self, id: usize, world_point: Point) -> Vector {
        self.normal_at_uv(id, world_point, 0.0, 0.0)
    }
    pub fn normal_at_uv(&self, id: usize, world_point: Point, u: Number, v: Number) -> Vector {
        let local_point = self.world_to_object(id, world_point);
        let local_normal = self.objects[id].local_normal_at_uv(&local_point, u, v);
        self.normal_to_world(id, local_normal)
    }
    fn surface_at(&self, comps: &Computations) -> Color {
        let object = &self.objects[comps.object_id];
        let mut surface = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        // Index loop over lights (no slice iterator) for rust-gpu. Light is Copy.
        let mut li = 0;
        while li < self.lights.len() {
            let light = self.lights[li];
            let intensity = self.intensity_at(comps.over_point, &light);
            surface = surface
                + lightning(
                    object,
                    light,
                    comps.point,
                    comps.eyev,
                    comps.normalv,
                    intensity,
                );
            li += 1;
        }
        surface
    }
    pub fn shade_hit(&self, comps: Computations, remaining: usize) -> Color {
        let surface = self.surface_at(&comps);
        let reflected = self.reflected_color(&comps, remaining);
        let refracted = self.refracted_color(&comps, remaining);

        let material = self.objects[comps.object_id].get_material();
        if material.reflective > 0.0 && material.transparency > 0.0 {
            let reflectance = comps.schlick();
            return surface + reflected * reflectance + refracted * (1.0 - reflectance);
        }
        surface + reflected + refracted
    }
    pub fn color_at(&self, ray: &Ray, remaining: usize) -> Color {
        let mut total = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        let mut stack = [ShadeJob::default(); MAX_SHADE_STACK];
        let mut sp = 0usize;
        stack[sp] = ShadeJob {
            ray: *ray,
            remaining,
            weight: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        };
        sp += 1;

        while sp > 0 {
            sp -= 1;
            let job = stack[sp];
            let xs = self.intersect_world(&job.ray);
            let hi = xs.hit_index();
            if hi == xs.len {
                continue;
            }
            let hit = xs.xs[hi];
            let comps = hit.prepare_computations(&job.ray, self, &xs);
            total = total + self.surface_at(&comps) * job.weight;

            if job.remaining == 0 {
                continue;
            }
            let material = self.objects[comps.object_id].get_material();
            let reflective = material.reflective;
            let transparency = material.transparency;
            if reflective == 0.0 && transparency == 0.0 {
                continue;
            }
            let cos_i = comps.eyev.dot(comps.normalv);
            let n_ratio = comps.n1 / comps.n2;
            let sin2_t = n_ratio.powi(2) * (1.0 - cos_i.powi(2));
            let tir = sin2_t > 1.0;

            let both = reflective > 0.0 && transparency > 0.0;
            let reflectance = if both { comps.schlick() } else { 1.0 };

            if reflective > 0.0 && sp < MAX_SHADE_STACK {
                let w = if both { reflective * reflectance } else { reflective };
                stack[sp] = ShadeJob {
                    ray: Ray {
                        origin: comps.over_point,
                        direction: comps.reflectv,
                    },
                    remaining: job.remaining - 1,
                    weight: job.weight * w,
                };
                sp += 1;
            }
            if transparency > 0.0 && !tir && sp < MAX_SHADE_STACK {
                let cos_t = (1.0 - sin2_t).sqrt();
                let direction =
                    comps.normalv * (n_ratio * cos_i - cos_t) - comps.eyev * n_ratio;
                let w = if both {
                    transparency * (1.0 - reflectance)
                } else {
                    transparency
                };
                stack[sp] = ShadeJob {
                    ray: Ray {
                        origin: comps.under_point,
                        direction,
                    },
                    remaining: job.remaining - 1,
                    weight: job.weight * w,
                };
                sp += 1;
            }
        }
        total
    }
    pub fn is_shadowed(&self, point: Point, light: &Light) -> bool {
        self.is_shadowed_at(light.position(), point)
    }
    pub fn is_shadowed_at(&self, light_position: Point, point: Point) -> bool {
        let v = light_position - point;
        let distance = v.magnitude();
        let direction = v.normalize();

        let r = Ray {
            origin: point,
            direction,
        };

        let xs = self.intersect_world(&r);
        let hi = xs.hit_index();
        if hi == xs.len {
            false
        } else {
            let t = xs.xs[hi].t;
            t > EPSILON && t < distance
        }
    }
    pub fn intensity_at(&self, point: Point, light: &Light) -> Number {
        if light.kind == 0 {
            if self.is_shadowed_at(light.position(), point) {
                0.0
            } else {
                1.0
            }
        } else {
            let mut total = 0.0;
            for v in 0..light.vsteps as usize {
                for u in 0..light.usteps as usize {
                    if !self.is_shadowed_at(light.point_on_light(u, v), point) {
                        total += 1.0;
                    }
                }
            }
            total / light.samples as Number
        }
    }
    pub fn reflected_color(&self, comps: &Computations, remaining: usize) -> Color {
        let material = self.objects[comps.object_id].get_material();
        if material.reflective == 0.0 || remaining <= 0 {
            return Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            };
        }
        let reflect_ray = Ray {
            origin: comps.over_point,
            direction: comps.reflectv,
        };
        let color = self.color_at(&reflect_ray, remaining - 1);
        color * material.reflective
    }
    pub fn refracted_color(&self, comps: &Computations, remaining: usize) -> Color {
        let object = &self.objects[comps.object_id];
        if object.get_material().transparency == 0.0 || remaining <= 0 {
            return Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            };
        }
        let n_ratio = comps.n1 / comps.n2;
        let cos_i = comps.eyev.dot(comps.normalv);
        let sin2_t = n_ratio.powi(2) * (1.0 - cos_i.powi(2));
        if sin2_t > 1.0 {
            return Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            };
        }
        let cos_t = (1.0 - sin2_t).sqrt();
        let direction = comps.normalv * (n_ratio * cos_i - cos_t) - comps.eyev * n_ratio;
        let refract_ray = Ray {
            origin: comps.under_point,
            direction,
        };
        self.color_at(&refract_ray, remaining - 1) * object.get_material().transparency
    }
}
#[cfg(feature = "std")]
impl Default for World {
    fn default() -> Self {
        let light = Light::point_light(Point {
                x: -10.0,
                y: 10.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let mut s1 = Primitive::sphere();
        let mut m1: Material = Material::default();
        m1.set_color(Color {
            r: 0.8,
            g: 1.0,
            b: 0.6,
        });
        m1.set_diffuse(0.7);
        m1.set_specular(0.2);
        s1.set_material(m1);

        let mut s2 = Primitive::sphere();
        const TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5);
        s2.set_transform(TRANSFORM);

        World {
            objects: vec![s1, s2],
            lights: vec![light],
            children: vec![vec![], vec![]],
            child_indices: vec![],
            use_bounds: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_a_world() {
        let w = World::new();
        assert_eq!(w.objects, vec![]);
        assert_eq!(w.lights, vec![]);
    }
    #[test]
    fn the_default_world() {
        let light = Light::point_light(Point {
                x: -10.0,
                y: 10.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let mut s1 = Primitive::sphere();
        let mut m1 = Material::default();
        m1.set_color(Color {
            r: 0.8,
            g: 1.0,
            b: 0.6,
        });
        m1.set_diffuse(0.7);
        m1.set_specular(0.2);
        s1.set_material(m1);

        let mut s2 = Primitive::sphere();
        const TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5);
        s2.set_transform(TRANSFORM);

        let w = World::default();
        assert_eq!(w.lights, vec![light]);
        assert_eq!(w.objects[0], s1);
        assert_eq!(w.objects[1], s2);
    }
    #[test]
    fn intersect_a_world_with_a_ray() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = w.intersect_world(&r);
        assert_eq!(xs.count(), 4);
        assert_eq!(xs[0].t, 4.0);
        assert_eq!(xs[1].t, 4.5);
        assert_eq!(xs[2].t, 5.5);
        assert_eq!(xs[3].t, 6.0);
    }
    #[test]
    fn shading_an_intersection() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let i = Intersection::new(4.0, 0);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        assert_eq!(
            w.shade_hit(comps, 0),
            Color {
                r: 0.38066,
                g: 0.47583,
                b: 0.2855
            }
        );
    }
    #[test]
    fn shading_an_intersection_from_the_inside() {
        let mut w = World::default();
        w.lights = vec![Light::point_light(Point {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            })];

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let i = Intersection::new(0.5, 1);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        assert_eq!(
            w.shade_hit(comps, 0),
            Color {
                r: 0.90498,
                g: 0.90498,
                b: 0.90498
            }
        );
    }
    #[test]
    fn the_color_when_a_ray_misses() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let c = w.color_at(&r, 0);
        assert_eq!(
            c,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }
    #[test]
    fn the_color_when_a_ray_hits() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let c = w.color_at(&r, 0);
        assert_eq!(
            c,
            Color {
                r: 0.38066,
                g: 0.47583,
                b: 0.2855
            }
        );
    }
    #[test]
    fn the_color_with_an_intersection_behind_the_ray() {
        let mut w = World::default();
        let mut object_material0 = w.objects[0].get_material();
        object_material0.set_ambient(1.0);
        w.objects[0].set_material(object_material0);
        let mut object_material1 = w.objects[1].get_material();
        object_material1.set_ambient(1.0);
        w.objects[1].set_material(object_material1);

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.75,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        };
        let c = w.color_at(&r, 0);
        assert_eq!(c, w.objects[1].get_material().color);
    }
    #[test]
    fn the_transformation_matrix_for_the_default_orientation() {
        let from = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Point {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        let up = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(t, Matrix::identity());
    }
    #[test]
    fn a_view_transformation_matrix_looking_in_positive_z_direction() {
        let from = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Point {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let up = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(t, scaling(-1.0, 1.0, -1.0));
    }
    #[test]
    fn the_view_transformation_moves_the_world() {
        let from = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let up = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(t, scaling(0.0, 0.0, -8.0));
    }
    #[test]
    fn an_arbitrary_view_transformation() {
        let from = Point {
            x: 1.0,
            y: 3.0,
            z: 2.0,
        };
        let to = Point {
            x: 4.0,
            y: -2.0,
            z: 8.0,
        };
        let up = Vector {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(
            t,
            Matrix::new([
                [-0.50709, 0.50709, 0.67612, -2.36643],
                [0.76772, 0.60609, 0.12122, -2.82843],
                [-0.35857, 0.59761, -0.71714, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ])
        );
    }
    #[test]
    fn the_area_light_intensity_function() {
        let w = World::default();
        let light = Light::area_light(
            Point {
                x: -0.5,
                y: -0.5,
                z: -5.0,
            },
            Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            2,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            2,
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        );
        let cases = [
            (Point { x: 0.0, y: 0.0, z: 2.0 }, 0.0),
            (Point { x: 1.0, y: -1.0, z: 2.0 }, 0.25),
            (Point { x: 1.5, y: 0.0, z: 2.0 }, 0.5),
            (Point { x: 1.25, y: 1.25, z: 3.0 }, 0.75),
            (Point { x: 0.0, y: 0.0, z: -2.0 }, 1.0),
        ];
        for (point, expected) in cases {
            assert_eq!(w.intensity_at(point, &light), expected, "point={point:?}");
        }
    }
    #[test]
    fn point_lights_evaluate_the_light_intensity_at_a_given_point() {
        let w = World::default();
        let cases = [
            (Point { x: 0.0, y: 1.0001, z: 0.0 }, 1.0),
            (Point { x: -1.0001, y: 0.0, z: 0.0 }, 1.0),
            (Point { x: 0.0, y: 0.0, z: -1.0001 }, 1.0),
            (Point { x: 0.0, y: 0.0, z: 1.0001 }, 0.0),
            (Point { x: 0.0, y: 0.0, z: 0.0 }, 0.0),
        ];
        let light = w.lights[0].clone();
        for (point, expected) in cases {
            assert_eq!(w.intensity_at(point, &light), expected, "point={point:?}");
        }
    }
    #[test]
    fn root_of_walks_to_the_top_level_object() {
        let mut w = World::new();
        let g1 = w.add_object(Primitive::group());
        let g2 = w.add_child(g1, Primitive::group());
        let s = w.add_child(g2, Primitive::sphere());
        assert_eq!(w.root_of(s), g1);
        assert_eq!(w.root_of(g2), g1);
        assert_eq!(w.root_of(g1), g1);
        // A lone root is its own root.
        let lone = w.add_object(Primitive::sphere());
        assert_eq!(w.root_of(lone), lone);
    }

    #[test]
    fn pickability_excludes_floors_and_the_sky() {
        let mut w = World::new();
        let sphere = w.add_object(Primitive::sphere()); // unit, extent 2 -> pickable
        assert!(w.is_pickable(sphere));
        let plane = w.add_object(Primitive::plane()); // infinite -> not pickable
        assert!(!w.is_pickable(plane));
        let mut huge = Primitive::sphere();
        huge.set_transform(scaling(1000.0, 1000.0, 1000.0)); // sky -> not pickable
        let sky = w.add_object(huge);
        assert!(!w.is_pickable(sky));
    }

    // Helper: the children ids of the group at `id`.
    fn group_children(w: &World, id: usize) -> Vec<usize> {
        if w.objects[id].kind == ShapeKind::Group {
            w.children[id].clone()
        } else {
            panic!("object {id} is not a group")
        }
    }

    #[test]
    fn partitioning_a_groups_children() {
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
        let mut s1 = Primitive::sphere();
        s1.set_transform(translation(-2.0, 0.0, 0.0));
        let s1 = w.add_child(g, s1);
        let mut s2 = Primitive::sphere();
        s2.set_transform(translation(2.0, 0.0, 0.0));
        let s2 = w.add_child(g, s2);
        let s3 = w.add_child(g, Primitive::sphere());
        let (left, right) = w.partition_children(g);
        assert_eq!(group_children(&w, g), vec![s3]);
        assert_eq!(left, vec![s1]);
        assert_eq!(right, vec![s2]);
    }

    #[test]
    fn creating_a_subgroup_from_a_list_of_children() {
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
        let s1 = w.add_object(Primitive::sphere());
        let s2 = w.add_object(Primitive::sphere());
        w.make_subgroup(g, vec![s1, s2]);
        let children = group_children(&w, g);
        assert_eq!(children.len(), 1);
        assert_eq!(group_children(&w, children[0]), vec![s1, s2]);
    }

    #[test]
    fn subdividing_a_group_partitions_its_children() {
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
        let mut s1 = Primitive::sphere();
        s1.set_transform(translation(-2.0, -2.0, 0.0));
        let s1 = w.add_child(g, s1);
        let mut s2 = Primitive::sphere();
        s2.set_transform(translation(-2.0, 2.0, 0.0));
        let s2 = w.add_child(g, s2);
        let mut s3 = Primitive::sphere();
        s3.set_transform(scaling(4.0, 4.0, 4.0));
        let s3 = w.add_child(g, s3);
        w.divide(g, 1);
        let children = group_children(&w, g);
        // The big sphere straddles the split and stays; the other two go into a
        // sub-group that is itself split one-per-child.
        assert_eq!(children[0], s3);
        let subgroup = children[1];
        let sub = group_children(&w, subgroup);
        assert_eq!(sub.len(), 2);
        assert_eq!(group_children(&w, sub[0]), vec![s1]);
        assert_eq!(group_children(&w, sub[1]), vec![s2]);
    }

    #[test]
    fn subdividing_a_group_with_too_few_children() {
        let mut w = World::new();
        let subgroup = w.add_object(Primitive::group());
        let mut s1 = Primitive::sphere();
        s1.set_transform(translation(-2.0, 0.0, 0.0));
        let s1 = w.add_child(subgroup, s1);
        let mut s2 = Primitive::sphere();
        s2.set_transform(translation(2.0, 1.0, 0.0));
        let s2 = w.add_child(subgroup, s2);
        let mut s3 = Primitive::sphere();
        s3.set_transform(translation(2.0, -1.0, 0.0));
        let s3 = w.add_child(subgroup, s3);
        // Hang the subgroup and a lone sphere under a parent group.
        let g = w.add_object(Primitive::group());
        w.objects[subgroup].set_parent(Some(g));
        if w.objects[g].kind == ShapeKind::Group {
            w.children[g].push(subgroup);
        }
        let s4 = w.add_child(g, Primitive::sphere());
        w.divide(g, 3);
        // g has 2 < 3 children, so it is left intact, but the subgroup (3) splits.
        assert_eq!(group_children(&w, g), vec![subgroup, s4]);
        let sub = group_children(&w, subgroup);
        assert_eq!(group_children(&w, sub[0]), vec![s1]);
        assert_eq!(group_children(&w, sub[1]), vec![s2, s3]);
    }

    #[test]
    fn subdividing_a_csg_shapes_children() {
        let mut w = World::new();
        let csg = w.add_object(Primitive::csg(crate::csg::CsgOperation::Difference));
        let left = w.add_object(Primitive::group());
        let mut s1 = Primitive::sphere();
        s1.set_transform(translation(-1.5, 0.0, 0.0));
        let s1 = w.add_child(left, s1);
        let mut s2 = Primitive::sphere();
        s2.set_transform(translation(1.5, 0.0, 0.0));
        let s2 = w.add_child(left, s2);
        let right = w.add_object(Primitive::group());
        let mut s3 = Primitive::sphere();
        s3.set_transform(translation(0.0, 0.0, -1.5));
        let s3 = w.add_child(right, s3);
        let mut s4 = Primitive::sphere();
        s4.set_transform(translation(0.0, 0.0, 1.5));
        let s4 = w.add_child(right, s4);
        w.set_csg_children(csg, left, right);
        w.divide(csg, 1);
        let left_children = group_children(&w, left);
        assert_eq!(group_children(&w, left_children[0]), vec![s1]);
        assert_eq!(group_children(&w, left_children[1]), vec![s2]);
        let right_children = group_children(&w, right);
        assert_eq!(group_children(&w, right_children[0]), vec![s3]);
        assert_eq!(group_children(&w, right_children[1]), vec![s4]);
    }

    #[test]
    fn there_is_no_shadow_when_nothing_is_collinear_with_point_and_light() {
        let w = World::default();
        let p = Point {
            x: 0.0,
            y: 10.0,
            z: 0.0,
        };
        assert_eq!(w.is_shadowed(p, &w.lights[0]), false);
    }
    #[test]
    fn the_shadow_when_an_object_is_between_the_point_and_the_light() {
        let w = World::default();
        let p = Point {
            x: 10.0,
            y: -10.0,
            z: 10.0,
        };
        assert_eq!(w.is_shadowed(p, &w.lights[0]), true);
    }
    #[test]
    fn there_is_no_shadow_when_an_object_is_behind_the_light() {
        let w = World::default();
        let p = Point {
            x: -20.0,
            y: 20.0,
            z: -20.0,
        };
        assert_eq!(w.is_shadowed(p, &w.lights[0]), false);
    }
    #[test]
    fn there_is_no_shadow_when_an_object_is_behind_the_point() {
        let w = World::default();
        let p = Point {
            x: -2.0,
            y: 2.0,
            z: -2.0,
        };
        assert_eq!(w.is_shadowed(p, &w.lights[0]), false);
    }
    #[test]
    fn shade_hit_is_given_an_intersection_in_shadow() {
        let mut w = World::default();
        let light = Light::point_light(Point {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        w.lights = vec![light];
        let s1 = Primitive::sphere();
        const TRANSFORM: Matrix<4, 4> = translation(0.0, 0.0, 10.0);
        let mut s2 = Primitive::sphere();
        s2.set_transform(TRANSFORM);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let i = Intersection::new(4.0, 1);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        w.objects.extend(vec![s1, s2.clone()]);
        let c = w.shade_hit(comps, 0);
        assert_eq!(
            c,
            Color {
                r: 0.1,
                g: 0.1,
                b: 0.1
            }
        );
    }
    #[test]
    fn the_hit_should_offset_the_point() {
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let mut shape = Primitive::sphere();
        const TRANSFORM: Matrix<4, 4> = translation(0.0, 0.0, 1.0);
        shape.set_transform(TRANSFORM);
        let i = Intersection::new(5.0, 0);
        let mut w = World::new();
        w.objects.append(&mut vec![shape]);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        assert_eq!(comps.over_point.z() < -EPSILON / 2.0, true);
        assert_eq!(comps.point.z() > comps.over_point.z(), true);
    }
    #[test]
    fn the_reflected_color_for_a_nonreflective_material() {
        let mut w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let mut second_object_material = w.objects[1].get_material();
        second_object_material.set_ambient(1.0);
        w.objects[1].set_material(second_object_material);
        let i = Intersection::new(1.0, 1);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        let color = w.reflected_color(&comps, 0);
        assert_eq!(
            color,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }
    #[test]
    fn the_reflected_color_for_a_reflective_material() {
        let mut w = World::default();
        let mut shape = Primitive::plane();
        let mut material = Material::default();
        material.set_reflective(0.5);
        const TRANSFORM: Matrix<4, 4> = Matrix::identity().then(translation(0.0, -1.0, 0.0));
        shape.set_material(material);
        shape.set_transform(TRANSFORM);
        w.objects.append(&mut vec![shape]);

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -3.0,
            },
            direction: Vector {
                x: 0.0,
                y: -sqrt(2.0) / 2.0,
                z: sqrt(2.0) / 2.0,
            },
        };

        let i = Intersection::new(sqrt(2.0), 2);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        let color = w.reflected_color(&comps, 1);
        // Book value, published to 5 decimals; compare within that precision.
        assert_almost_eq!(color.r, 0.19032, 1e-4);
        assert_almost_eq!(color.g, 0.2379, 1e-4);
        assert_almost_eq!(color.b, 0.14274, 1e-4);
    }
    #[test]
    fn shade_hit_with_a_reflective_material() {
        let mut w = World::default();
        let mut shape = Primitive::plane();
        let mut material = shape.get_material().clone();
        material.set_reflective(0.5);
        shape.set_material(material);
        shape.set_transform(translation(0.0, -1.0, 0.0));
        w.objects.append(&mut vec![shape]);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -3.0,
            },
            direction: Vector {
                x: 0.0,
                y: -sqrt(2.0) / 2.0,
                z: sqrt(2.0) / 2.0,
            },
        };
        let i = Intersection::new(sqrt(2.0), 2);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        let color = w.shade_hit(comps, 1);
        // Book value, published to 5 decimals; compare within that precision.
        assert_almost_eq!(color.r, 0.87677, 1e-4);
        assert_almost_eq!(color.g, 0.92436, 1e-4);
        assert_almost_eq!(color.b, 0.82918, 1e-4);
    }
    #[test]
    fn color_at_with_mutally_reflective_surfaces() {
        let mut w = World::default();
        w.lights = vec![Light::point_light(
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        )];

        let mut lower = Primitive::plane();
        let mut lower_material = lower.get_material().clone();
        lower_material.set_reflective(1.0);
        lower.set_transform(translation(0.0, -1.0, 0.0));
        lower.set_material(lower_material);
        let mut upper = Primitive::plane();
        let mut upper_material = upper.get_material().clone();
        upper_material.set_reflective(1.0);
        upper.set_transform(translation(0.0, 1.0, 0.0));
        upper.set_material(upper_material);
        w.objects.append(&mut vec![lower, upper]);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let color = w.color_at(&r, 5);
        assert_eq!(
            color,
            Color {
                r: 1.9,
                g: 1.9,
                b: 1.9
            }
        )
    }
    #[test]
    fn the_refracted_color_with_an_opaque_surface() {
        let w = World::default();
        let _shape = w.objects[0].clone();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = Intersections::new(vec![Intersection::new(4.0, 0), Intersection::new(6.0, 0)]);
        let comps = xs[0].prepare_computations(&r, &w.scene(), &xs);
        let c = w.refracted_color(&comps, 5);
        assert_eq!(
            c,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }
    #[test]
    fn the_refracted_color_at_the_maximum_recursive_depth() {
        let mut w = World::default();
        w.objects[0] = Primitive::glass_sphere();

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = Intersections::new(vec![Intersection::new(4.0, 0), Intersection::new(6.0, 0)]);
        let comps = xs[0].prepare_computations(&r, &w.scene(), &xs);
        let c = w.refracted_color(&comps, 0);
        assert_eq!(
            c,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }
    #[test]
    fn the_refracted_color_under_total_internal_reflection() {
        let mut w = World::default();
        w.objects[0] = Primitive::glass_sphere();

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: sqrt(2.0) / 2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let xs = Intersections::new(vec![
            Intersection::new(-sqrt(2.0) / 2.0, 0),
            Intersection::new(sqrt(2.0) / 2.0, 0),
        ]);
        let comps = xs[1].prepare_computations(&r, &w.scene(), &xs);
        let c = w.refracted_color(&comps, 5);
        assert_eq!(
            c,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }

    #[test]
    fn the_refracted_color_with_a_refracted_ray() {
        let mut w = World::default();
        let mut a_material = Material::default();
        a_material.set_ambient(1.0);
        a_material.set_pattern(Pattern::test_pattern());

        let a = Primitive::with(Primitive::sphere, Matrix::identity(), a_material);

        w.objects[0] = a;
        w.objects[1] = Primitive::glass_sphere();

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.1,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let xs = Intersections::new(vec![
            Intersection::new(-0.9899, 0),
            Intersection::new(-0.4899, 1),
            Intersection::new(0.4899, 1),
            Intersection::new(0.9899, 0),
        ]);
        let comps = xs[2].prepare_computations(&r, &w.scene(), &xs);
        let c = w.refracted_color(&comps, 5);
        // Book value, published to 5 decimals; compare within that precision.
        assert_almost_eq!(c.r, 0.0, 1e-4);
        assert_almost_eq!(c.g, 0.99888, 1e-4);
        assert_almost_eq!(c.b, 0.04725, 1e-4);
    }
    #[test]
    fn shade_hit_with_a_transparent_material() {
        let mut w = World::default();
        let mut glass = Material::default();
        glass.set_transparency(0.5);
        glass.set_refractive_index(1.5);

        let floor = Primitive::with(Primitive::plane, translation(0.0, -1.0, 0.0), glass);
        let mut ball_material = Material::default();
        ball_material.set_color(Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
        });
        ball_material.set_ambient(0.5);
        let ball = Primitive::with(Primitive::sphere, translation(0.0, -3.5, -0.5), ball_material);
        w.objects.append(&mut vec![floor, ball]);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -3.0,
            },
            direction: Vector {
                x: 0.0,
                y: -sqrt(2.0) / 2.0,
                z: sqrt(2.0) / 2.0,
            },
        };
        let xs = Intersections::new(vec![Intersection::new(sqrt(2.0), 2)]);
        let comps = xs[0].prepare_computations(&r, &w.scene(), &xs);
        let color = w.shade_hit(comps, 5);
        assert_eq!(
            color,
            Color {
                r: 0.93642,
                g: 0.68642,
                b: 0.68642
            }
        )
    }
    #[test]
    fn shade_hit_with_a_reflective_transparent_material() {
        let mut w = World::default();
        let mut glass = Material::default();
        glass.set_transparency(0.5);
        glass.set_reflective(0.5);
        glass.set_refractive_index(1.5);

        let floor = Primitive::with(Primitive::plane, translation(0.0, -1.0, 0.0), glass);
        let mut ball_material = Material::default();
        ball_material.set_color(Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
        });
        ball_material.set_ambient(0.5);
        let ball = Primitive::with(Primitive::sphere, translation(0.0, -3.5, -0.5), ball_material);
        w.objects.append(&mut vec![floor, ball]);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -3.0,
            },
            direction: Vector {
                x: 0.0,
                y: -sqrt(2.0) / 2.0,
                z: sqrt(2.0) / 2.0,
            },
        };
        let xs = Intersections::new(vec![Intersection::new(sqrt(2.0), 2)]);
        let comps = xs[0].prepare_computations(&r, &w.scene(), &xs);
        let color = w.shade_hit(comps, 5);
        assert_eq!(
            color,
            Color {
                r: 0.93391,
                g: 0.69643,
                b: 0.69243
            }
        )
    }
}
