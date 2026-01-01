use crate::intersections;
use crate::intersections::*;
use crate::matrices::*;
use crate::rays::*;
use crate::transformations::*;
use crate::tuples::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Sphere {
    pub origin: Tuple,
    pub radius: f32,
    pub transform: Matrix<4, 4>,
}

impl Sphere {
    pub const fn new(origin: Tuple, radius: f32, transform: Matrix<4, 4>) -> Self {
        Self {
            origin,
            radius,
            transform,
        }
    }
    pub const fn unit() -> Self {
        Self::new(Tuple::point(0.0, 0.0, 0.0), 1.0, Matrix::identity())
    }
    pub fn intersect(&self, ray: &Ray) -> Vec<Intersection> {
        let ray2 = match inverse(&self.transform) {
            None => ray,
            Some(m) => &ray.transform(m),
        };

        let sphere_to_ray = ray2.origin - self.origin;

        let a = dot(&ray2.direction, &ray2.direction);
        let b = 2.0 * dot(&ray2.direction, &sphere_to_ray);
        let c = dot(&sphere_to_ray, &sphere_to_ray) - 1.0;

        let discriminant = b.powi(2) - 4.0 * a * c;

        if discriminant < 0.0 {
            return vec![];
        }

        let t1 = (-b - discriminant.sqrt()) / (2.0 * a);
        let t2 = (-b + discriminant.sqrt()) / (2.0 * a);

        let i1 = Intersection::new(t1, self.clone());
        let i2 = Intersection::new(t2, self.clone());
        intersections(&[i1, i2])
    }
    pub fn set_transform(&mut self, transform: &Matrix<4, 4>) -> () {
        self.transform = transform.clone();
    }
}

#[test]
fn a_ray_intersects_a_sphere_at_two_points() {
    const R: Ray = Ray::new(Tuple::point(0.0, 0.0, -5.0), Tuple::vector(0.0, 0.0, 1.0));
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, 4.0);
    assert_eq!(xs[1].t, 6.0);
}
#[test]
fn a_ray_intersects_a_sphere_at_a_tangent() {
    const R: Ray = Ray::new(Tuple::point(0.0, 1.0, -5.0), Tuple::vector(0.0, 0.0, 1.0));
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, 5.0);
    assert_eq!(xs[1].t, 5.0);
}
#[test]
fn a_ray_misses_a_sphere() {
    const R: Ray = Ray::new(Tuple::point(0.0, 2.0, -5.0), Tuple::vector(0.0, 0.0, 1.0));
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs.iter().count(), 0);
}
#[test]
fn a_ray_originates_inside_a_sphere() {
    const R: Ray = Ray::new(Tuple::point(0.0, 0.0, 0.0), Tuple::vector(0.0, 0.0, 1.0));
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, -1.0);
    assert_eq!(xs[1].t, 1.0);
}
#[test]
fn a_sphere_is_behind_a_ray() {
    const R: Ray = Ray::new(Tuple::point(0.0, 0.0, 5.0), Tuple::vector(0.0, 0.0, 1.0));
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, -6.0);
    assert_eq!(xs[1].t, -4.0);
}
#[test]
fn intersect_sets_the_object_on_the_intersection() {
    const R: Ray = Ray::new(Tuple::point(0.0, 0.0, -5.0), Tuple::vector(0.0, 0.0, 1.0));
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    intersections(&[xs[0], xs[1]]);
    assert_eq!(xs[0].object, S);
}
#[test]
fn a_spheres_default_transformation() {
    const S: Sphere = Sphere::unit();
    assert_eq!(S.transform, Matrix::identity());
}
#[test]
fn changing_a_spheres_transformation() {
    let mut s: Sphere = Sphere::unit();
    const T: Matrix<4, 4> = translation(2.0, 3.0, 4.0);
    s.set_transform(&T);
    assert_eq!(s.transform, T);
}
#[test]
fn intersecting_a_scaled_sphere_with_a_ray() {
    const R: Ray = Ray::new(Tuple::point(0.0, 0.0, -5.0), Tuple::vector(0.0, 0.0, 1.0));
    let mut s = Sphere::unit();
    s.set_transform(&scaling(2.0, 2.0, 2.0));
    let xs = s.intersect(&R);
    assert_eq!(xs.iter().count(), 2);
    assert_eq!(xs[0].t, 3.0);
    assert_eq!(xs[1].t, 7.0);
}
#[test]
fn intersecting_a_translated_sphere_with_a_ray() {
    const R: Ray = Ray::new(Tuple::point(0.0, 0.0, 0.0), Tuple::vector(0.0, 0.0, 1.0));
    let mut s = Sphere::unit();
    s.set_transform(&translation(5.0, 0.0, 0.0));
    let xs = s.intersect(&R);
    assert_eq!(xs.iter().count(), 0);
}
