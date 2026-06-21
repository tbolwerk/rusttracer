use crate::{
    bounds::BoundingBox, cones::Cone, cubes::Cube, cylinders::Cylinder, groups::*, intersections::*,
    materials::Material, matrices::*, planes::Plane, rays::*, spheres::Sphere, tuples::*,
};
use std::sync::{Arc, Mutex};

macro_rules! shape_match {
    ($self:expr, $binding:ident => $body:expr) => {
        match $self {
            Shape::Test($binding) => $body,
            Shape::Sphere($binding) => $body,
            Shape::Plane($binding) => $body,
            Shape::Cube($binding) => $body,
            Shape::Cylinder($binding) => $body,
            Shape::Cone($binding) => $body,
            Shape::Group($binding) => $body,
        }
    };
}
#[derive(Debug, Clone)]
pub(crate) struct TestShape {
    transform: TransformData,
    material: Material,
    saved_ray: Arc<Mutex<Option<Ray>>>,
}

impl TestShape {
    pub fn saved_ray(&self) -> Option<Ray> {
        self.saved_ray.lock().unwrap().clone()
    }
}
impl PartialEq for TestShape {
    fn eq(&self, _other: &TestShape) -> bool {
        unreachable!()
    }
}

impl Default for TestShape {
    fn default() -> Self {
        Self {
            transform: TransformData {
                transform: Matrix::identity(),
                inverse_transform: None,
                parent: None,
            },
            material: Material::default(),
            saved_ray: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Shape {
    Test(TestShape),
    Sphere(Sphere),
    Plane(Plane),
    Cube(Cube),
    Cylinder(Cylinder),
    Cone(Cone),
    Group(Group),
}
impl Shape {
    fn test_shape() -> Shape {
        Shape::Test(TestShape::default())
    }
    pub const fn sphere() -> Shape {
        Shape::Sphere(Sphere::unit())
    }
    pub fn cube() -> Shape {
        Shape::Cube(Cube::default())
    }
    pub fn cylinder() -> Shape {
        Shape::Cylinder(Cylinder::default())
    }
    pub fn cone() -> Shape {
        Shape::Cone(Cone::default())
    }
    pub fn glass_sphere() -> Shape {
        let mut sphere = Shape::Sphere(Sphere::unit());
        let mut glass = Material::default();
        glass.set_transparency(1.0);
        glass.set_refractive_index(1.5);
        sphere.set_material(glass);
        sphere
    }
    pub fn plane() -> Shape {
        Shape::Plane(Plane::default())
    }
    pub fn group() -> Shape {
        Shape::Group(Group::new())
    }
    // The group this shape belongs to (an index into `World::objects`), or
    // `None` if it is a root. Mirrors the book's `parent` attribute.
    pub fn parent(&self) -> Option<usize> {
        shape_match!(self, s => s.transform.get_parent())
    }
    pub fn set_parent(&mut self, parent: Option<usize>) {
        shape_match!(self, s => s.transform.set_parent(parent))
    }
    // The shape's normal in its own object space. Lifting it into world space
    // (accounting for any enclosing groups) is done by `World::normal_at`.
    pub fn local_normal_at(&self, point: &Point) -> Vector {
        shape_match!(self, s => s.local_normal_at(point))
    }
    // The shape's axis-aligned bounding box in its own object space, before its
    // transform is applied. `World::compute_bounds` lifts these into group
    // space to build each group's enclosing box. A group has no geometry of its
    // own, so it returns an empty box here; its real bounds (the union of its
    // children) are cached on the group by `World::compute_bounds`.
    pub fn local_bounds(&self) -> BoundingBox {
        match self {
            Shape::Plane(_) => BoundingBox::new(
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
            Shape::Cylinder(c) => BoundingBox::new(
                Point {
                    x: -1.0,
                    y: c.minimum,
                    z: -1.0,
                },
                Point {
                    x: 1.0,
                    y: c.maximum,
                    z: 1.0,
                },
            ),
            Shape::Cone(c) => {
                let limit = c.minimum.abs().max(c.maximum.abs());
                BoundingBox::new(
                    Point {
                        x: -limit,
                        y: c.minimum,
                        z: -limit,
                    },
                    Point {
                        x: limit,
                        y: c.maximum,
                        z: limit,
                    },
                )
            }
            Shape::Group(_) => BoundingBox::empty(),
            // Sphere, Cube and the test shape all fit the unit cube.
            _ => BoundingBox::new(
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
    pub fn with(shape: fn() -> Shape, transform: Matrix<4, 4>, material: Material) -> Shape {
        let mut s = shape();
        s.set_transform(transform);
        s.set_material(material);
        s
    }
    pub fn intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        let local_ray = match self.get_inverse_transform() {
            None => ray,
            Some(inverse_transform) => &ray.transform(inverse_transform),
        };
        shape_match!(self, s => s.local_intersect(&local_ray, object_id))
    }
    pub fn normal_at(&self, point: &Point) -> Vector {
        let inverse_transform = match self.get_inverse_transform() {
            None => Matrix::identity(),
            Some(inverse_transform) => inverse_transform,
        };
        let local_point = inverse_transform * point.clone();
        let local_normal = shape_match!(self, s => s.local_normal_at(&local_point));
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

pub trait Intersects {
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections;
    fn local_normal_at(&self, point: &Point) -> Vector {
        Vector {
            x: point.x(),
            y: point.y(),
            z: point.z(),
        }
    }
}
impl HasTransform for Shape {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        shape_match!(self, s => s.transform.set_transform(transform))
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        shape_match!(self, s => s.transform.get_transform())
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        shape_match!(self, s => s.transform.get_inverse_transform())
    }
}
impl HasMaterial for Shape {
    fn set_material(&mut self, material: Material) -> () {
        shape_match!(self, s => s.set_material(material))
    }
    fn get_material(&self) -> Material {
        shape_match!(self, s => s.get_material())
    }
}
impl Intersects for TestShape {
    fn local_intersect(&self, ray: &Ray, _: usize) -> Intersections {
        let mut reference = self.saved_ray.lock().unwrap();
        *reference = Some(ray.clone());
        Intersections::new(vec![])
    }
}
impl HasMaterial for TestShape {
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
        let s = Shape::test_shape();
        assert_eq!(s.get_transform(), Matrix::identity());
    }
    #[test]
    fn assigning_a_transformation() {
        let mut s = Shape::test_shape();
        s.set_transform(translation(2.0, 3.0, 4.0));
        assert_eq!(s.get_transform(), translation(2.0, 3.0, 4.0));
    }
    #[test]
    fn the_default_material() {
        let s = Shape::test_shape();
        assert_eq!(s.get_material(), Material::default());
    }
    #[test]
    fn assigning_a_material() {
        let mut s = Shape::test_shape();
        let mut m = Material::default();
        m.set_ambient(1.0);
        s.set_material(m.clone());
        assert_eq!(s.get_material(), m);
    }
    #[test]
    fn intersecting_a_scaled_shape_with_a_ray() {
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
        let mut s = Shape::test_shape();
        s.set_transform(scaling(2.0, 2.0, 2.0));
        let _ = s.intersect(&r, 0);
        let saved_ray = match s {
            Shape::Test(test_shape) => test_shape.saved_ray(),
            _ => None,
        }
        .unwrap();
        assert_eq!(
            saved_ray.origin,
            Point {
                x: 0.0,
                y: 0.0,
                z: -2.5
            }
        );
        assert_eq!(
            saved_ray.direction,
            Vector {
                x: 0.0,
                y: 0.0,
                z: 0.5
            }
        )
    }
    #[test]
    fn intersecting_a_translated_shape_with_a_ray() {
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
        let mut s = Shape::test_shape();
        s.set_transform(translation(5.0, 0.0, 0.0));
        let _ = s.intersect(&r, 0);
        let saved_ray = match s {
            Shape::Test(test_shape) => test_shape.saved_ray(),
            _ => None,
        }
        .unwrap();
        assert_eq!(
            saved_ray.origin,
            Point {
                x: -5.0,
                y: 0.0,
                z: -5.0
            }
        );
        assert_eq!(
            saved_ray.direction,
            Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0
            }
        );
    }
    #[test]
    fn computing_the_normal_on_a_translated_shape() {
        let mut s = Shape::test_shape();
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
        let mut s = Shape::test_shape();
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
