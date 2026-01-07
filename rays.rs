use crate::{matrices::Matrix, transformations::*, tuples::*};
#[derive(Debug, PartialEq, Clone)]
pub struct Ray {
    pub origin: Point,
    pub direction: Vector,
}

impl Ray {
    pub fn position(&self, t: f32) -> Point {
        self.origin + self.direction * t
    }
    pub fn transform(&self, t: Matrix<4, 4>) -> Self {
        Self {
            origin: t * self.origin,
            direction: t * self.direction,
        }
    }
}
mod tests {
    use super::*;
    #[test]
    fn creating_and_querying_a_ray() {
        let origin = Point {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let direction = Vector {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let r = Ray {
            origin: origin,
            direction: direction,
        };
        assert_eq!(r.origin, origin);
        assert_eq!(r.direction, direction);
    }
    #[test]
    fn computing_a_point_from_a_distance() {
        let r = Ray {
            origin: Point {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            },
            direction: Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        };
        assert_eq!(
            r.position(0.0),
            Point {
                x: 2.0,
                y: 3.0,
                z: 4.0
            }
        );
        assert_eq!(
            r.position(1.0),
            Point {
                x: 3.0,
                y: 3.0,
                z: 4.0
            }
        );
        assert_eq!(
            r.position(-1.0),
            Point {
                x: 1.0,
                y: 3.0,
                z: 4.0
            }
        );
        assert_eq!(
            r.position(2.5),
            Point {
                x: 4.5,
                y: 3.0,
                z: 4.0
            }
        );
    }
    #[test]
    fn translating_a_ray() {
        let r = Ray {
            origin: Point {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        const M: Matrix<4, 4> = translation(3.0, 4.0, 5.0);
        let r2 = r.transform(M);
        assert_eq!(
            r2.origin,
            Point {
                x: 4.0,
                y: 6.0,
                z: 8.0
            }
        );
        assert_eq!(
            r2.direction,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn scaling_a_ray() {
        let r = Ray {
            origin: Point {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        const M: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
        let r2: Ray = r.transform(M);
        assert_eq!(
            r2.origin,
            Point {
                x: 2.0,
                y: 6.0,
                z: 12.0
            }
        );
        assert_eq!(
            r2.direction,
            Vector {
                x: 0.0,
                y: 3.0,
                z: 0.0
            }
        );
    }
}
