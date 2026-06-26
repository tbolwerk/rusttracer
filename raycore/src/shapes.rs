use crate::{
    bounds::BoundingBox,
    cones::cone_normal_at,
    cones::cone_intersect,
    csg::CsgOperation,
    cubes::{cube_intersect, cube_normal_at},
    cylinders::{cylinder_intersect, cylinder_normal_at},
    intersections::*,
    materials::Material,
    matrices::*,
    planes::{plane_intersect, plane_normal_at},
    rays::*,
    spheres::{sphere_intersect, sphere_normal_at},
    triangles::{
        smooth_triangle_local_normal_at_uv, triangle_intersect, triangle_normal_at,
    },
    tuples::*,
};

// The flat shape kind tag. The renderer dispatches geometry by matching on this
// instead of an enum carrying per-shape data; all the data now lives in the
// flat `Primitive` struct alongside the tag. This keeps the type a plain
// data-only struct, which the rust-gpu/SPIR-V backend can handle.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeKind {
    Sphere,
    Plane,
    Cube,
    Cylinder,
    Cone,
    Triangle,
    SmoothTriangle,
    Group,
    Csg,
}

// A single shape, flat. Every field for every kind lives here; a given kind
// only reads the fields it cares about and leaves the rest at their defaults.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Primitive {
    pub kind: ShapeKind,
    pub transform: TransformData,
    pub material: Material,
    // cylinder / cone
    pub minimum: Number,
    pub maximum: Number,
    pub closed: u32,
    // triangle / smooth triangle
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
    pub e1: Vector,
    pub e2: Vector,
    pub normal: Vector,
    // smooth triangle vertex normals
    pub n1: Vector,
    pub n2: Vector,
    pub n3: Vector,
    // group: children live in `World::child_indices[child_start .. child_start +
    // child_count]` (a flat, heap-free projection the trace reads), not on the
    // primitive. The logical adjacency is kept in `World::children`.
    pub child_start: u32,
    pub child_count: u32,
    // csg. left/right are arena indices into `World::objects`; the sentinel
    // `u32::MAX` means "unset" (no child). The Option-returning accessors
    // `left()`/`right()` translate this for the rest of the code.
    pub operation: CsgOperation,
    pub left: u32,
    pub right: u32,
    // cached group/csg bounds. `has_bounds` is the sentinel flag standing in for
    // the old `Option`: when false `bounds` is meaningless. Use `bounds()` /
    // `set_bounds()` instead of touching the fields directly.
    pub bounds: BoundingBox,
    pub has_bounds: u32,
}

// Sentinel for `left`/`right`: no child attached. (CSG nodes set both; every
// other kind leaves them at this.)
const NO_CHILD: u32 = u32::MAX;

// Hand-written to mirror the old `Option<BoundingBox>` field: the cached bounds
// only participate in equality when both sides actually have them. An unset box
// is `BoundingBox::empty()` (min +inf / max -inf), and the tolerant `Point`
// equality computes `inf - inf = NaN`, which compares unequal, so deriving
// `PartialEq` would make two equal "no-bounds" primitives compare unequal. The
// `bounds()` accessor returns `None` in that case, matching the old behavior.
impl PartialEq for Primitive {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.transform == other.transform
            && self.material == other.material
            && self.minimum == other.minimum
            && self.maximum == other.maximum
            && self.closed == other.closed
            && self.p1 == other.p1
            && self.p2 == other.p2
            && self.p3 == other.p3
            && self.e1 == other.e1
            && self.e2 == other.e2
            && self.normal == other.normal
            && self.n1 == other.n1
            && self.n2 == other.n2
            && self.n3 == other.n3
            && self.child_start == other.child_start
            && self.child_count == other.child_count
            && self.operation == other.operation
            && self.left == other.left
            && self.right == other.right
            && self.bounds() == other.bounds()
    }
}

impl Primitive {
    // Build a primitive of `kind` with every geometry field at its default; each
    // per-kind constructor then sets only what it needs.
    fn base(kind: ShapeKind) -> Self {
        let zero = Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let origin = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        Self {
            kind,
            transform: TransformData::default(),
            material: Material::default(),
            minimum: 0.0,
            maximum: 0.0,
            closed: 0,
            p1: origin,
            p2: origin,
            p3: origin,
            e1: zero,
            e2: zero,
            normal: zero,
            n1: zero,
            n2: zero,
            n3: zero,
            child_start: 0,
            child_count: 0,
            operation: CsgOperation::Union,
            left: NO_CHILD,
            right: NO_CHILD,
            bounds: BoundingBox::empty(),
            has_bounds: 0,
        }
    }
    pub fn sphere() -> Primitive {
        Self::base(ShapeKind::Sphere)
    }
    pub fn cube() -> Primitive {
        Self::base(ShapeKind::Cube)
    }
    pub fn cylinder() -> Primitive {
        let mut p = Self::base(ShapeKind::Cylinder);
        p.minimum = Number::MIN;
        p.maximum = Number::MAX;
        p.closed = 0;
        p
    }
    pub fn cone() -> Primitive {
        let mut p = Self::base(ShapeKind::Cone);
        p.minimum = Number::MIN;
        p.maximum = Number::MAX;
        p.closed = 0;
        p
    }
    pub fn glass_sphere() -> Primitive {
        let mut sphere = Self::sphere();
        let mut glass = Material::default();
        glass.set_transparency(1.0);
        glass.set_refractive_index(1.5);
        sphere.set_material(glass);
        sphere
    }
    pub fn plane() -> Primitive {
        Self::base(ShapeKind::Plane)
    }
    pub fn group() -> Primitive {
        Self::base(ShapeKind::Group)
    }
    // A CSG node with its children unset; attach them with `World::set_csg_children`.
    pub fn csg(operation: CsgOperation) -> Primitive {
        let mut p = Self::base(ShapeKind::Csg);
        p.operation = operation;
        p
    }
    pub fn triangle(p1: Point, p2: Point, p3: Point) -> Primitive {
        let e1 = p2 - p1;
        let e2 = p3 - p1;
        // The triangle is flat, so a single normal serves every point on it.
        let normal = e2.cross(e1).normalize();
        let mut p = Self::base(ShapeKind::Triangle);
        p.p1 = p1;
        p.p2 = p2;
        p.p3 = p3;
        p.e1 = e1;
        p.e2 = e2;
        p.normal = normal;
        p
    }
    pub fn smooth_triangle(
        p1: Point,
        p2: Point,
        p3: Point,
        n1: Vector,
        n2: Vector,
        n3: Vector,
    ) -> Primitive {
        let mut p = Self::base(ShapeKind::SmoothTriangle);
        p.p1 = p1;
        p.p2 = p2;
        p.p3 = p3;
        p.e1 = p2 - p1;
        p.e2 = p3 - p1;
        p.n1 = n1;
        p.n2 = n2;
        p.n3 = n3;
        p
    }
    // The group this shape belongs to (an index into `World::objects`), or
    // `None` if it is a root. Mirrors the book's `parent` attribute.
    pub fn parent(&self) -> Option<usize> {
        self.transform.get_parent()
    }
    pub fn set_parent(&mut self, parent: Option<usize>) {
        self.transform.set_parent(parent)
    }
    // CSG children, as Option<usize> over the flat `u32` sentinel fields. The
    // rest of the code keeps using these instead of the raw fields.
    pub fn left(&self) -> Option<usize> {
        if self.left == NO_CHILD {
            None
        } else {
            Some(self.left as usize)
        }
    }
    pub fn right(&self) -> Option<usize> {
        if self.right == NO_CHILD {
            None
        } else {
            Some(self.right as usize)
        }
    }
    pub fn set_left(&mut self, id: usize) {
        self.left = id as u32;
    }
    pub fn set_right(&mut self, id: usize) {
        self.right = id as u32;
    }
    // Cached group/CSG bounds, as Option over the `has_bounds` sentinel. Returned
    // BY VALUE (BoundingBox is Copy) because rust-gpu can't lower `Option<&T>`.
    pub fn bounds(&self) -> Option<BoundingBox> {
        if self.has_bounds != 0 {
            Some(self.bounds)
        } else {
            None
        }
    }
    pub fn set_bounds(&mut self, bounds: BoundingBox) {
        self.bounds = bounds;
        self.has_bounds = 1;
    }
    // The shape's normal in its own object space. Lifting it into world space
    // (accounting for any enclosing groups) is done by `World::normal_at`.
    pub fn local_normal_at(&self, point: &Point) -> Vector {
        self.local_normal_at_uv(point, 0.0, 0.0)
    }
    // u/v-aware object-space normal. Forwarded by `World::normal_at_uv`; only a
    // smooth triangle consults u/v, the rest ignore them.
    pub fn local_normal_at_uv(&self, point: &Point, u: Number, v: Number) -> Vector {
        match self.kind {
            ShapeKind::Sphere => sphere_normal_at(point),
            ShapeKind::Plane => plane_normal_at(point),
            ShapeKind::Cube => cube_normal_at(point),
            ShapeKind::Cylinder => cylinder_normal_at(self, point),
            ShapeKind::Cone => cone_normal_at(self, point),
            ShapeKind::Triangle => triangle_normal_at(self),
            ShapeKind::SmoothTriangle => smooth_triangle_local_normal_at_uv(self, u, v),
            // Groups and CSG nodes have no surface; the normal is resolved on the
            // hit leaf by `World::normal_at`, so this never runs for them.
            ShapeKind::Group | ShapeKind::Csg => Vector {
                x: point.x(),
                y: point.y(),
                z: point.z(),
            },
        }
    }
    // The shape's axis-aligned bounding box in its own object space, before its
    // transform is applied. `World::compute_bounds` lifts these into group
    // space to build each group's enclosing box. A group has no geometry of its
    // own, so it returns an empty box here; its real bounds (the union of its
    // children) are cached on the group by `World::compute_bounds`.
    pub fn local_bounds(&self) -> BoundingBox {
        match self.kind {
            ShapeKind::Plane => BoundingBox::new(
                Point {
                    x: Number::NEG_INFINITY,
                    y: 0.0,
                    z: Number::NEG_INFINITY,
                },
                Point {
                    x: Number::INFINITY,
                    y: 0.0,
                    z: Number::INFINITY,
                },
            ),
            ShapeKind::Cylinder => BoundingBox::new(
                Point {
                    x: -1.0,
                    y: self.minimum,
                    z: -1.0,
                },
                Point {
                    x: 1.0,
                    y: self.maximum,
                    z: 1.0,
                },
            ),
            ShapeKind::Cone => {
                let limit = self.minimum.abs().max(self.maximum.abs());
                BoundingBox::new(
                    Point {
                        x: -limit,
                        y: self.minimum,
                        z: -limit,
                    },
                    Point {
                        x: limit,
                        y: self.maximum,
                        z: limit,
                    },
                )
            }
            // Like a group, a CSG node's real box (the union of its children) is
            // cached on the node by `World::compute_bounds`.
            ShapeKind::Group | ShapeKind::Csg => BoundingBox::empty(),
            // A triangle's box is just the corner-wise min and max of its three
            // vertices; a smooth triangle shares the same three corners.
            ShapeKind::Triangle | ShapeKind::SmoothTriangle => {
                let mut b = BoundingBox::empty();
                b.add_point(self.p1);
                b.add_point(self.p2);
                b.add_point(self.p3);
                b
            }
            // Sphere and Cube both fit the unit cube.
            ShapeKind::Sphere | ShapeKind::Cube => BoundingBox::new(
                Point {
                    x: -1.0,
                    y: -1.0,
                    z: -1.0,
                },
                Point {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            ),
        }
    }
    pub fn with(
        shape: fn() -> Primitive,
        transform: Matrix<4, 4>,
        material: Material,
    ) -> Primitive {
        let mut s = shape();
        s.set_transform(transform);
        s.set_material(material);
        s
    }
    pub fn intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        let mut xs = Intersections::empty();
        self.intersect_into(ray, object_id, &mut xs);
        xs
    }
    // Push this leaf's intersections into `xs` (the buffer threaded through the
    // iterative world traversal). Applies the leaf's own inverse transform, then
    // dispatches on `kind`. Groups/CSG are handled by `World::intersect_object`.
    pub fn intersect_into(&self, ray: &Ray, object_id: usize, xs: &mut Intersections) {
        let local_ray = match self.get_inverse_transform() {
            None => ray.clone(),
            Some(inverse_transform) => ray.transform(inverse_transform),
        };
        match self.kind {
            ShapeKind::Sphere => sphere_intersect(&local_ray, object_id, xs),
            ShapeKind::Plane => plane_intersect(&local_ray, object_id, xs),
            ShapeKind::Cube => cube_intersect(&local_ray, object_id, xs),
            ShapeKind::Cylinder => cylinder_intersect(self, &local_ray, object_id, xs),
            ShapeKind::Cone => cone_intersect(self, &local_ray, object_id, xs),
            ShapeKind::Triangle => triangle_intersect(self, &local_ray, object_id, xs),
            ShapeKind::SmoothTriangle => triangle_intersect(self, &local_ray, object_id, xs),
            // Groups and CSG nodes are traversed by `World::intersect_object`,
            // never dispatched here.
            ShapeKind::Group | ShapeKind::Csg => {}
        }
    }
    pub fn normal_at(&self, point: &Point) -> Vector {
        let inverse_transform = match self.get_inverse_transform() {
            None => Matrix::identity(),
            Some(inverse_transform) => inverse_transform,
        };
        let local_point = inverse_transform * point.clone();
        let local_normal = self.local_normal_at(&local_point);
        let world_normal = transpose(&inverse_transform) * local_normal;
        world_normal.normalize()
    }
}

pub trait HasTransform {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> ();
    fn get_transform(&self) -> Matrix<4, 4>;
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>>;
}

// Sentinel for `parent`: this shape is a top-level (root) object, with no
// enclosing group.
const NO_PARENT: u32 = u32::MAX;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct TransformData {
    transform: Matrix<4, 4>,
    // The inverse of `transform`, always materialized (identity when the
    // transform is the identity, which is the no-op the old `None` stood for).
    // Flat, not Option, so the struct uploads to a SPIR-V buffer.
    inverse: Matrix<4, 4>,
    // Index into `World::objects` of the group this shape belongs to, or
    // `NO_PARENT` for a top-level (root) shape. This replaces the book's upward
    // parent pointer with an arena index; the Option API is preserved by
    // `get_parent`/`set_parent`.
    parent: u32,
}

impl TransformData {
    pub fn new(transform: Matrix<4, 4>, inverse_transform: Option<Matrix<4, 4>>) -> Self {
        Self {
            transform,
            inverse: inverse_transform.unwrap_or(Matrix::identity()),
            parent: NO_PARENT,
        }
    }
    pub fn get_parent(&self) -> Option<usize> {
        if self.parent == NO_PARENT {
            None
        } else {
            Some(self.parent as usize)
        }
    }
    pub fn set_parent(&mut self, parent: Option<usize>) {
        self.parent = parent.map(|x| x as u32).unwrap_or(NO_PARENT);
    }
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            transform: Matrix::identity(),
            inverse: Matrix::identity(),
            parent: NO_PARENT,
        }
    }
}

impl HasTransform for TransformData {
    fn set_transform(&mut self, transform: crate::matrices::Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse = crate::matrices::inverse(&transform).unwrap_or(Matrix::identity());
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        self.transform
    }
    // Always `Some`: the inverse is materialized (identity for an identity
    // transform). The old `None` meant "no transform", and identity is its
    // no-op equivalent, so callers that match on `None` still behave identically.
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        Some(self.inverse)
    }
}

impl HasMaterial for Material {
    fn set_material(&mut self, material: Material) -> () {
        *self = material;
    }
    fn get_material(&self) -> Material {
        self.clone()
    }
}

pub trait HasMaterial {
    fn set_material(&mut self, material: Material) -> ();
    fn get_material(&self) -> Material;
}

impl HasTransform for Primitive {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        self.transform.set_transform(transform)
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        self.transform.get_transform()
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        self.transform.get_inverse_transform()
    }
}
impl HasMaterial for Primitive {
    fn set_material(&mut self, material: Material) -> () {
        self.material = material;
    }
    fn get_material(&self) -> Material {
        self.material.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformations::{rotation_z, scaling, translation, PI};

    #[test]
    fn the_default_transformation() {
        let s = Primitive::sphere();
        assert_eq!(s.get_transform(), Matrix::identity());
    }
    #[test]
    fn assigning_a_transformation() {
        let mut s = Primitive::sphere();
        s.set_transform(translation(2.0, 3.0, 4.0));
        assert_eq!(s.get_transform(), translation(2.0, 3.0, 4.0));
    }
    #[test]
    fn the_default_material() {
        let s = Primitive::sphere();
        assert_eq!(s.get_material(), Material::default());
    }
    #[test]
    fn assigning_a_material() {
        let mut s = Primitive::sphere();
        let mut m = Material::default();
        m.set_ambient(1.0);
        s.set_material(m.clone());
        assert_eq!(s.get_material(), m);
    }
    #[test]
    fn intersecting_a_scaled_shape_with_a_ray() {
        // The transform must be applied (ray moved into object space) before the
        // local intersection. A unit sphere scaled by 2 along z, hit head-on from
        // z=-5, is struck at t=3 and t=7 (instead of 4 and 6 unscaled).
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
        let s = Primitive::with(
            Primitive::sphere,
            scaling(2.0, 2.0, 2.0),
            Material::default(),
        );
        let xs = s.intersect(&r, 0);
        assert_eq!(xs.count(), 2);
        assert_eq!(xs[0].t, 3.0);
        assert_eq!(xs[1].t, 7.0);
    }
    #[test]
    fn intersecting_a_translated_shape_with_a_ray() {
        // Translating the sphere +5 in x moves it out of the ray's path entirely,
        // so the transformed ray misses: zero intersections.
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
        let s = Primitive::with(
            Primitive::sphere,
            translation(5.0, 0.0, 0.0),
            Material::default(),
        );
        let xs = s.intersect(&r, 0);
        assert_eq!(xs.count(), 0);
    }
    #[test]
    fn computing_the_normal_on_a_translated_shape() {
        let mut s = Primitive::sphere();
        s.set_transform(translation(0.0, 1.0, 0.0));
        let n = s.normal_at(&Point {
            x: 0.0,
            y: 1.70711,
            z: -0.70711,
        });
        assert_eq!(
            n,
            Vector {
                x: 0.0,
                y: 0.7071068,
                z: -0.70710677
            },
        )
    }
    #[test]
    fn computing_the_normal_on_a_transformed_shape() {
        let mut s = Primitive::sphere();
        s.set_transform(rotation_z(PI / 5.0).then(scaling(1.0, 0.5, 1.0)));
        let n = s.normal_at(&Point {
            x: 0.0,
            y: sqrt(2.0) / 2.0,
            z: -sqrt(2.0) / 2.0,
        });
        assert_eq!(
            n,
            Vector {
                x: 0.0,
                y: 0.97014,
                z: -0.24254
            }
        )
    }
}
