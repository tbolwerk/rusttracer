use crate::{
    intersections::*,
    materials::Material,
    matrices::*,
    planes::Plane,
    rays::*,
    spheres::Sphere,
    transformations::{rotation_z, scaling, translation, PI},
    tuples::*,
};
use std::sync::{Arc, Mutex};
#[derive(Debug, Clone)]
struct TestShape {
    transform: Matrix<4, 4>,
    inverse_transform: Option<Matrix<4, 4>>,
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
            transform: Matrix::identity(),
            inverse_transform: None,
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
}

impl Shape {
    fn test_shape() -> Shape {
        Shape::Test(TestShape::default())
    }
    pub const fn sphere() -> Shape {
        Shape::Sphere(Sphere::unit())
    }
    pub fn plane() -> Shape {
        Shape::Plane(Plane::default())
    }
    pub fn intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        let local_ray = match self.get_inverse_transform() {
            None => ray,
            Some(inverse_transform) => &ray.transform(inverse_transform),
        };
        match self {
            Shape::Test(test_shape) => test_shape.local_intersect(&local_ray, object_id),
            Shape::Sphere(sphere) => sphere.local_intersect(&local_ray, object_id),
            Shape::Plane(plane) => plane.local_intersect(&local_ray, object_id),
        }
    }
    pub fn normal_at(&self, point: &Point) -> Vector {
        let inverse_transform = match self.get_inverse_transform() {
            None => Matrix::identity(),
            Some(inverse_transform) => inverse_transform,
        };
        let local_point = inverse_transform * point.clone();
        let local_normal = match self {
            Shape::Test(test_shape) => test_shape.local_normal_at(&local_point),
            Shape::Sphere(sphere) => sphere.local_normal_at(&local_point),
            Shape::Plane(plane) => plane.local_normal_at(&local_point),
        };
        let world_normal: Vector = transpose(&inverse_transform) * local_normal;
        world_normal.normalize()
    }
}

pub trait HasTransform {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> ();
    fn get_transform(&self) -> Matrix<4, 4>;
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>>;
}

pub trait HasMaterial {
    fn set_material(&mut self, material: Material) -> ();
    fn get_material(&self) -> Material;
}

pub trait Intersects: HasMaterial + HasTransform {
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections;
    fn local_normal_at(&self, point: &Point) -> Vector {
        Vector {
            x: point.x(),
            y: point.y(),
            z: point.z(),
        }
    }
}

impl HasTransform for TestShape {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
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

impl HasTransform for Shape {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        match self {
            Shape::Test(test_shape) => test_shape.set_transform(transform),
            Shape::Sphere(sphere) => sphere.set_transform(transform),
            Shape::Plane(plane) => plane.set_transform(transform),
        }
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        match self {
            Shape::Test(test_shape) => test_shape.get_transform(),
            Shape::Sphere(sphere) => sphere.transform,
            Shape::Plane(plane) => plane.get_transform(),
        }
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        match self {
            Shape::Test(test_shape) => test_shape.get_inverse_transform(),
            Shape::Sphere(sphere) => sphere.inverse_transform,
            Shape::Plane(plane) => plane.get_inverse_transform(),
        }
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
impl HasMaterial for Shape {
    fn set_material(&mut self, material: Material) -> () {
        match self {
            Shape::Test(test_shape) => test_shape.set_material(material),
            Shape::Sphere(sphere) => sphere.set_material(material),
            Shape::Plane(plane) => plane.set_material(material),
        }
    }
    fn get_material(&self) -> Material {
        match self {
            Shape::Test(test_shape) => test_shape.get_material(),
            Shape::Sphere(sphere) => sphere.material.clone(),
            Shape::Plane(plane) => plane.get_material(),
        }
    }
}
impl Intersects for TestShape {
    fn local_intersect(&self, ray: &Ray, _: usize) -> Intersections {
        let mut reference = self.saved_ray.lock().unwrap();
        *reference = Some(ray.clone());
        Intersections::new(vec![])
    }
}

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
        y: 2.0_f32.sqrt() / 2.0,
        z: -2.0_f32.sqrt() / 2.0,
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
