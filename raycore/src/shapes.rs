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
#[derive(Debug, Clone, PartialEq)]
pub struct Primitive {
    pub kind: ShapeKind,
    pub transform: TransformData,
    pub material: Material,
    // cylinder / cone
    pub minimum: Number,
    pub maximum: Number,
    pub closed: bool,
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
    // group
    pub children: Vec<usize>,
    // csg
    pub operation: CsgOperation,
    pub left: Option<usize>,
    pub right: Option<usize>,
    // cached group/csg bounds
    pub bounds: Option<BoundingBox>,
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
            closed: false,
            p1: origin,
            p2: origin,
            p3: origin,
            e1: zero,
            e2: zero,
            normal: zero,
            n1: zero,
            n2: zero,
            n3: zero,
            children: vec![],
            operation: CsgOperation::Union,
            left: None,
            right: None,
            bounds: None,
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
        p.closed = false;
        p
    }
    pub fn cone() -> Primitive {
        let mut p = Self::base(ShapeKind::Cone);
        p.minimum = Number::MIN;
        p.maximum = Number::MAX;
        p.closed = false;
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
        let local_ray = match self.get_inverse_transform() {
            None => ray.clone(),
            Some(inverse_transform) => ray.transform(inverse_transform),
        };
        match self.kind {
            ShapeKind::Sphere => sphere_intersect(&local_ray, object_id),
            ShapeKind::Plane => plane_intersect(&local_ray, object_id),
            ShapeKind::Cube => cube_intersect(&local_ray, object_id),
            ShapeKind::Cylinder => cylinder_intersect(self, &local_ray, object_id),
            ShapeKind::Cone => cone_intersect(self, &local_ray, object_id),
            ShapeKind::Triangle => triangle_intersect(self, &local_ray, object_id),
            ShapeKind::SmoothTriangle => triangle_intersect(self, &local_ray, object_id),
            // Groups and CSG nodes are traversed by `World::intersect_object`,
            // never dispatched here.
            ShapeKind::Group | ShapeKind::Csg => Intersections::new(vec![]),
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

#[derive(Clone, Debug, PartialEq)]
pub struct TransformData {
    transform: Matrix<4, 4>,
    inverse_transform: Option<Matrix<4, 4>>,
    // Index into `World::objects` of the group this shape belongs to, if any.
    // `None` means this is a top-level (root) shape. This replaces the book's
    // upward parent pointer with an arena index.
    parent: Option<usize>,
}

impl TransformData {
    pub const fn new(transform: Matrix<4, 4>, inverse_transform: Option<Matrix<4, 4>>) -> Self {
        Self {
            transform,
            inverse_transform,
            parent: None,
        }
    }
    pub fn get_parent(&self) -> Option<usize> {
        self.parent
    }
    pub fn set_parent(&mut self, parent: Option<usize>) {
        self.parent = parent;
    }
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            transform: Matrix::identity(),
            inverse_transform: None,
            parent: None,
        }
    }
}

impl HasTransform for TransformData {
    fn set_transform(&mut self, transform: crate::matrices::Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse_transform = inverse(&transform);
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        self.transform
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        self.inverse_transform
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
