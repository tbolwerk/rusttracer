use crate::{matrices::Matrix, transformations::*, tuples::*};

pub struct Ray {
    pub origin: Tuple,
    pub direction: Tuple,
}

impl Ray {
    pub const fn new(origin: Tuple, direction: Tuple) -> Self {
        Self { origin, direction }
    }
    pub fn position(&self, t: f32) -> Tuple {
        self.origin + self.direction * t
    }
    pub fn transform(&self, t: Matrix<4, 4>) -> Self {
        Self::new(t * self.origin, t * self.direction)
    }
}

#[test]
fn creating_and_querying_a_ray() {
    const ORIGIN: Tuple = Tuple::point(1.0, 2.0, 3.0);
    const DIRECTION: Tuple = Tuple::vector(4.0, 5.0, 6.0);
    const R: Ray = Ray::new(ORIGIN, DIRECTION);
    assert_eq!(R.origin, ORIGIN);
    assert_eq!(R.direction, DIRECTION);
}
#[test]
fn computing_a_point_from_a_distance() {
    const R: Ray = Ray::new(Tuple::point(2.0, 3.0, 4.0), Tuple::vector(1.0, 0.0, 0.0));
    assert_eq!(R.position(0.0), Tuple::point(2.0, 3.0, 4.0));
    assert_eq!(R.position(1.0), Tuple::point(3.0, 3.0, 4.0));
    assert_eq!(R.position(-1.0), Tuple::point(1.0, 3.0, 4.0));
    assert_eq!(R.position(2.5), Tuple::point(4.5, 3.0, 4.0));
}
#[test]
fn translating_a_ray() {
    const R: Ray = Ray::new(Tuple::point(1.0, 2.0, 3.0), Tuple::vector(0.0, 1.0, 0.0));
    const M: Matrix<4, 4> = translation(3.0, 4.0, 5.0);
    let r2: Ray = R.transform(M);
    assert_eq!(r2.origin, Tuple::point(4.0, 6.0, 8.0));
    assert_eq!(r2.direction, Tuple::vector(0.0, 1.0, 0.0));
}
#[test]
fn scaling_a_ray() {
    const R: Ray = Ray::new(Tuple::point(1.0, 2.0, 3.0), Tuple::vector(0.0, 1.0, 0.0));
    const M: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
    let r2: Ray = R.transform(M);
    assert_eq!(r2.origin, Tuple::point(2.0, 6.0, 12.0));
    assert_eq!(r2.direction, Tuple::vector(0.0, 3.0, 0.0));
}
