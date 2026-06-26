use crate::bounds::BoundingBox;
use crate::csg::intersection_allowed;
use crate::intersections::Computations;
#[cfg(test)]
use crate::intersections::Intersection;
use crate::intersections::Intersections;
use crate::lights::*;
use crate::materials::lightning;
use crate::materials::Material;
use crate::matrices::transpose;
use crate::matrices::Matrix;
#[cfg(test)]
use crate::patterns::*;
use crate::rays::Ray;
use crate::shapes::*;
use crate::transformations::*;
use crate::tuples::*;

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub objects: Vec<Primitive>,
    pub lights: Vec<Light>,
    // When true, a group culls its children against its bounding box before
    // recursing. Always correct to leave on; exposed only so a scene can render
    // the same world with it off to measure the speedup.
    pub use_bounds: bool,
}
impl World {
    pub fn new() -> Self {
        Self {
            objects: vec![],
            lights: vec![],
            use_bounds: true,
        }
    }
    pub fn intersect_world(&self, ray: &Ray) -> Intersections {
        let mut intersections = Intersections {
            intersections: vec![],
        };
        // Only roots are traversed here; children are reached recursively by
        // intersect_object, so a child must not be intersected a second time.
        for (id, object) in self.objects.iter().enumerate() {
            if object.parent().is_none() {
                intersections.extend(self.intersect_object(id, ray));
            }
        }
        // Sort once, here, now that every root has contributed. `color_at` and the
        // tests rely on `intersect_world` returning hits in t-order.
        intersections.sort();
        intersections
    }
    // Dispatch a ray to the arena object `id`. For a group, move the ray into
    // the group's space and recurse into its children. For a leaf, hand off to
    // the primitive's own `Primitive::intersect`, which applies the leaf's
    // transform. Transforms therefore compose down the hierarchy exactly as in
    // the book's Group::intersect.
    pub fn intersect_object(&self, id: usize, ray: &Ray) -> Intersections {
        let object = &self.objects[id];
        match object.kind {
            ShapeKind::Group => {
                let local_ray = match object.get_inverse_transform() {
                    None => ray.clone(),
                    Some(inverse) => ray.transform(inverse),
                };
                // Bounding-box cull: if the ray (in group space) misses the
                // group's box, none of its children can be hit, so skip them
                // all. `bounds` is None until `compute_bounds` runs, in which
                // case we fall through and test every child as before.
                if self.use_bounds {
                    if let Some(bounds) = &object.bounds {
                        if !bounds.intersects(&local_ray) {
                            return Intersections::new(vec![]);
                        }
                    }
                }
                let mut xs = Intersections::new(vec![]);
                for &child in &object.children {
                    xs.extend(self.intersect_object(child, &local_ray));
                }
                xs
            }
            ShapeKind::Csg => {
                let local_ray = match object.get_inverse_transform() {
                    None => ray.clone(),
                    Some(inverse) => ray.transform(inverse),
                };
                // Same bounding-box cull as a group: miss the box, skip both
                // children and the filtering entirely.
                if self.use_bounds {
                    if let Some(bounds) = &object.bounds {
                        if !bounds.intersects(&local_ray) {
                            return Intersections::new(vec![]);
                        }
                    }
                }
                // Intersect both children, then keep only the hits the operation
                // allows. `extend` re-sorts, so the combined list is in t-order,
                // which `filter_intersections` relies on.
                let mut xs = self.intersect_object(object.left.unwrap(), &local_ray);
                xs.extend(self.intersect_object(object.right.unwrap(), &local_ray));
                self.filter_intersections(id, xs)
            }
            _ => object.intersect(ray, id),
        }
    }
    // Whether the subtree rooted at `node` contains the leaf `object`. A CSG uses
    // this to decide whether a hit belongs to its left or right child, even when
    // that child is itself a group or CSG. Mirrors the book's `includes`.
    pub fn includes(&self, node: usize, object: usize) -> bool {
        if node == object {
            return true;
        }
        let node_obj = &self.objects[node];
        match node_obj.kind {
            ShapeKind::Group => node_obj
                .children
                .iter()
                .any(|&c| self.includes(c, object)),
            ShapeKind::Csg => {
                node_obj.left.map_or(false, |l| self.includes(l, object))
                    || node_obj.right.map_or(false, |r| self.includes(r, object))
            }
            _ => false,
        }
    }
    // Keep only the intersections that lie on the CSG's combined surface. Walking
    // the hits in t-order, track whether the ray is currently inside the left and
    // right children; for each hit, `intersection_allowed` decides if it survives,
    // then crossing that surface flips the corresponding inside flag.
    pub fn filter_intersections(&self, csg_id: usize, mut xs: Intersections) -> Intersections {
        let csg = &self.objects[csg_id];
        let (operation, left) = match csg.kind {
            ShapeKind::Csg => (csg.operation, csg.left.unwrap()),
            _ => return xs,
        };
        // The walk below depends on t-order, and the two children were appended
        // without sorting, so order them now.
        xs.sort();
        let mut inside_left = false;
        let mut inside_right = false;
        let mut result = vec![];
        for intersection in &xs.intersections {
            let left_hit = self.includes(left, intersection.object_id);
            if intersection_allowed(operation, left_hit, inside_left, inside_right) {
                result.push(*intersection);
            }
            if left_hit {
                inside_left = !inside_left;
            } else {
                inside_right = !inside_right;
            }
        }
        Intersections::new(result)
    }
    // Object `id`'s bounding box in its own space, computed from scratch by
    // recursing into children (a leaf's `local_bounds`, a group/CSG's union of
    // its children's parent-space boxes). Unlike the cached `bounds`, this does
    // not depend on `compute_bounds` having run or on arena id ordering, so it is
    // safe to call mid-`divide` while the hierarchy is being rebuilt.
    fn object_bounds(&self, id: usize) -> BoundingBox {
        let obj = &self.objects[id];
        let children: Vec<usize> = match obj.kind {
            ShapeKind::Group => obj.children.clone(),
            ShapeKind::Csg => [obj.left, obj.right].into_iter().flatten().collect(),
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
    }
    // Cache the box for `id` (if it is a group/CSG) and return its own-space box,
    // computing children first.
    fn compute_bounds_of(&mut self, id: usize) -> BoundingBox {
        let obj = &self.objects[id];
        let children: Vec<usize> = match obj.kind {
            ShapeKind::Group => obj.children.clone(),
            ShapeKind::Csg => [obj.left, obj.right].into_iter().flatten().collect(),
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
            ShapeKind::Group | ShapeKind::Csg => obj.bounds = Some(bb),
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
            self.objects[id].children.clone()
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
            self.objects[id].children = kept;
        }
        (left, right)
    }
    // Wrap `children` in a new sub-group and attach it to group `id`. The book's
    // `make_subgroup`.
    fn make_subgroup(&mut self, id: usize, children: Vec<usize>) {
        let subgroup = self.objects.len();
        self.objects.push(Primitive::group());
        self.objects[subgroup].set_parent(Some(id));
        for child in &children {
            self.objects[*child].set_parent(Some(subgroup));
        }
        if self.objects[subgroup].kind == ShapeKind::Group {
            self.objects[subgroup].children = children;
        }
        if self.objects[id].kind == ShapeKind::Group {
            self.objects[id].children.push(subgroup);
        }
    }
    // Recursively subdivide group `id` into a bounding-volume hierarchy: when a
    // group has at least `threshold` children, partition them into two halves and
    // tuck each half into its own sub-group, then recurse. A CSG node has no
    // children to partition, so it just forwards `divide` to its two operands.
    // The book's `divide`. Run `compute_bounds` afterward to cache the new boxes.
    pub fn divide(&mut self, id: usize, threshold: usize) {
        match self.objects[id].kind {
            ShapeKind::Group => {
                if threshold <= self.objects[id].children.len() {
                    let (left, right) = self.partition_children(id);
                    if !left.is_empty() {
                        self.make_subgroup(id, left);
                    }
                    if !right.is_empty() {
                        self.make_subgroup(id, right);
                    }
                }
                let children = self.objects[id].children.clone();
                for child in children {
                    self.divide(child, threshold);
                }
            }
            ShapeKind::Csg => {
                let (left, right) = (self.objects[id].left, self.objects[id].right);
                if let Some(left) = left {
                    self.divide(left, threshold);
                }
                if let Some(right) = right {
                    self.divide(right, threshold);
                }
            }
            _ => {}
        }
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
        let point = match self.objects[id].parent() {
            Some(parent) => self.world_to_object(parent, point),
            None => point,
        };
        let inverse = self.objects[id]
            .get_inverse_transform()
            .unwrap_or(Matrix::identity());
        inverse * point
    }
    // Book's normal_to_world: lift a normal out through this object's transform,
    // then each ancestor's, normalizing at every step.
    fn normal_to_world(&self, id: usize, normal: Vector) -> Vector {
        let inverse = self.objects[id]
            .get_inverse_transform()
            .unwrap_or(Matrix::identity());
        let normal = (transpose(&inverse) * normal).normalize();
        match self.objects[id].parent() {
            Some(parent) => self.normal_to_world(parent, normal),
            None => normal,
        }
    }
    // Book's normal_at: the world-space normal of the leaf `id` at `world_point`,
    // accounting for every enclosing group's transform.
    pub fn normal_at(&self, id: usize, world_point: Point) -> Vector {
        self.normal_at_uv(id, world_point, 0.0, 0.0)
    }
    // As `normal_at`, but carrying the hit's barycentric u/v so a smooth triangle
    // can interpolate its normal from its three vertex normals. All other shapes
    // ignore u/v, so `normal_at` just calls this with zeros.
    pub fn normal_at_uv(&self, id: usize, world_point: Point, u: Number, v: Number) -> Vector {
        let local_point = self.world_to_object(id, world_point);
        let local_normal = self.objects[id].local_normal_at_uv(&local_point, u, v);
        self.normal_to_world(id, local_normal)
    }
    // Append a top-level object and return its arena id.
    pub fn add_object(&mut self, object: Primitive) -> usize {
        let id = self.objects.len();
        self.objects.push(object);
        id
    }
    // Append `child` and attach it to the group at `group_id`: set the child's
    // parent and record its id in the group's children. Mirrors the book's
    // Group::add_child.
    pub fn add_child(&mut self, group_id: usize, mut child: Primitive) -> usize {
        child.set_parent(Some(group_id));
        let id = self.objects.len();
        self.objects.push(child);
        if self.objects[group_id].kind == ShapeKind::Group {
            self.objects[group_id].children.push(id);
        }
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
            self.objects[csg_id].left = Some(left);
            self.objects[csg_id].right = Some(right);
        }
    }
    pub fn shade_hit(&self, comps: Computations, remaining: usize) -> Color {
        let object = &self.objects[comps.object_id];
        // Sum the contribution of every light. Each light is shadow-tested
        // independently, so a point can be lit by one light while shadowed
        // from another. Note the material's ambient term is included once per
        // light, so multiple lights brighten ambient additively.
        let mut surface = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        for light in &self.lights {
            let intensity = self.intensity_at(comps.over_point, light);
            surface = surface
                + lightning(
                    &object,
                    light.clone(),
                    comps.point,
                    comps.eyev,
                    comps.normalv,
                    intensity,
                );
        }
        let reflected = self.reflected_color(&comps, remaining);
        let refracted = self.refracted_color(&comps, remaining);

        let material = object.get_material();
        if material.reflective > 0.0 && material.transparency > 0.0 {
            let reflectance = comps.schlick();
            return surface + reflected * reflectance + refracted * (1.0 - reflectance);
        }
        surface + reflected + refracted
    }
    pub fn color_at(&self, ray: &Ray, remaining: usize) -> Color {
        let xs = self.intersect_world(&ray);
        match xs.hit() {
            None => Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            Some(intersection) => self.shade_hit(
                intersection.prepare_computations(&ray, self, &xs),
                remaining,
            ),
        }
    }
    pub fn is_shadowed(&self, point: Point, light: &Light) -> bool {
        self.is_shadowed_at(light.position(), point)
    }
    // Is `point` occluded from `light_position` by some object between them? The
    // book's area-light version of `is_shadowed`, taking an explicit light point
    // so each sample of an area light can be tested independently.
    pub fn is_shadowed_at(&self, light_position: Point, point: Point) -> bool {
        let v = light_position - point;
        let distance = v.magnitude();
        let direction = v.normalize();

        let r = Ray {
            origin: point,
            direction,
        };

        match self.intersect_world(&r).hit() {
            None => false,
            Some(intersection) => intersection.t > EPSILON && intersection.t < distance,
        }
    }
    // The fraction of `light` visible from `point`: 1.0 or 0.0 for a point light,
    // and the share of unoccluded grid samples for an area light, which is what
    // produces soft-edged shadows.
    pub fn intensity_at(&self, point: Point, light: &Light) -> Number {
        match light {
            Light::Point(_) => {
                if self.is_shadowed_at(light.position(), point) {
                    0.0
                } else {
                    1.0
                }
            }
            Light::Area(area) => {
                let mut total = 0.0;
                for v in 0..area.vsteps {
                    for u in 0..area.usteps {
                        if !self.is_shadowed_at(area.point_on_light(u, v), point) {
                            total += 1.0;
                        }
                    }
                }
                total / area.samples as Number
            }
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
impl Default for World {
    fn default() -> Self {
        let light = Light::Point(PointLight {
            position: Point {
                x: -10.0,
                y: 10.0,
                z: -10.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
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
        let light = Light::Point(PointLight {
            position: Point {
                x: -10.0,
                y: 10.0,
                z: -10.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
        w.lights = vec![Light::Point(PointLight {
            position: Point {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
            w.objects[id].children.clone()
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
            w.objects[g].children.push(subgroup);
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
        let light = Light::Point(PointLight {
            position: Point {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
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
        let comps = xs[0].prepare_computations(&r, &w, &xs);
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
        let comps = xs[0].prepare_computations(&r, &w, &xs);
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
        let comps = xs[1].prepare_computations(&r, &w, &xs);
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
        let comps = xs[2].prepare_computations(&r, &w, &xs);
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
        let comps = xs[0].prepare_computations(&r, &w, &xs);
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
        let comps = xs[0].prepare_computations(&r, &w, &xs);
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
