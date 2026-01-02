use crate::{matrices::Matrix, transformations::*, tuples::external_tuples::*};

pub struct Ray {
    pub origin: TupleKind,
    pub direction: TupleKind,
}

impl Ray {
    pub const fn new(origin: TupleKind, direction: TupleKind) -> Self {
        Self { origin, direction }
    }
    pub fn position(&self, t: f32) -> TupleKind {
        self.origin + self.direction * t
    }
    pub fn transform(&self, t: Matrix<4, 4>) -> Self {
        Self::new(t * self.origin, t * self.direction)
    }
}

#[test]
fn creating_and_querying_a_ray() {
    const ORIGIN: TupleKind = TupleKind::point(1.0, 2.0, 3.0);
    const DIRECTION: TupleKind = TupleKind::vector(4.0, 5.0, 6.0);
    const R: Ray = Ray::new(ORIGIN, DIRECTION);
    assert_eq!(R.origin, ORIGIN);
    assert_eq!(R.direction, DIRECTION);
}
#[test]
fn computing_a_point_from_a_distance() {
    const R: Ray = Ray::new(
        TupleKind::point(2.0, 3.0, 4.0),
        TupleKind::vector(1.0, 0.0, 0.0),
    );
    assert_eq!(R.position(0.0), TupleKind::point(2.0, 3.0, 4.0));
    assert_eq!(R.position(1.0), TupleKind::point(3.0, 3.0, 4.0));
    assert_eq!(R.position(-1.0), TupleKind::point(1.0, 3.0, 4.0));
    assert_eq!(R.position(2.5), TupleKind::point(4.5, 3.0, 4.0));
}
#[test]
fn translating_a_ray() {
    const R: Ray = Ray::new(
        TupleKind::point(1.0, 2.0, 3.0),
        TupleKind::vector(0.0, 1.0, 0.0),
    );
    const M: Matrix<4, 4> = translation(3.0, 4.0, 5.0);
    let r2: Ray = R.transform(M);
    assert_eq!(r2.origin, TupleKind::point(4.0, 6.0, 8.0));
    assert_eq!(r2.direction, TupleKind::vector(0.0, 1.0, 0.0));
}
#[test]
fn scaling_a_ray() {
    const R: Ray = Ray::new(
        TupleKind::point(1.0, 2.0, 3.0),
        TupleKind::vector(0.0, 1.0, 0.0),
    );
    const M: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
    let r2: Ray = R.transform(M);
    assert_eq!(r2.origin, TupleKind::point(2.0, 6.0, 12.0));
    assert_eq!(r2.direction, TupleKind::vector(0.0, 3.0, 0.0));
}
