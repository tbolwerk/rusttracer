use crate::tuples::*;

pub struct Ray {
    origin: Tuple,
    direction: Tuple,
}

impl Ray {
    pub const fn new(origin: Tuple, direction: Tuple) -> Self {
        Self { origin, direction }
    }
}

#[test]
fn creating_and_querying_a_ray() {
    let origin = Tuple::point(1.0, 2.0, 3.0);
    let direction = Tuple::vector(4.0, 5.0, 6.0);
    let r = Ray::new(origin, direction);
    assert_eq!(r.origin, origin);
    assert_eq!(r.direction, direction);
}
