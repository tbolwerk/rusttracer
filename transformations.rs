use crate::matrices::{inverse, Matrix};
use crate::tuples::*;

pub const PI: f32 = 3.14159265;
const TWO_PI: f32 = 6.28318531;

pub const fn radians(degree: f32) -> f32 {
    degree * PI / 180.0
}

pub const fn translation(x: f32, y: f32, z: f32) -> Matrix<4, 4> {
    let mut m = Matrix::identity();
    m.set(0, 3, x);
    m.set(1, 3, y);
    m.set(2, 3, z);
    m
}

pub const fn scaling(x: f32, y: f32, z: f32) -> Matrix<4, 4> {
    let mut m = Matrix::init(0.0);
    m.set(0, 0, x);
    m.set(1, 1, y);
    m.set(2, 2, z);
    m.set(3, 3, 1.0);
    m
}

const fn normalize(mut x: f32) -> f32 {
    // Bring angle into the range of -PI to PI where Taylor series is stable
    while x > PI {
        x -= TWO_PI;
    }
    while x < -PI {
        x += TWO_PI;
    }
    x
}

const fn cos(x: f32) -> f32 {
    let x = normalize(x);
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;
    let x8 = x4 * x4;
    // Taylor series: 1 - x^2/2! + x^4/4! - x^6/6! + x^8/8!
    1.0 - x2 / 2.0 + x4 / 24.0 - x6 / 720.0 + x8 / 40320.0
}

const fn sin(x: f32) -> f32 {
    let x = normalize(x);
    let x2 = x * x;
    let x3 = x * x2;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    let x9 = x7 * x2;
    // Taylor series: x - x^3/3! + x^5/5! - x^7/7! + x^9/9!
    x - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0 + x9 / 362880.0
}

pub const fn rotation_x(r: f32) -> Matrix<4, 4> {
    let mut m = Matrix::identity();
    m.set(1, 1, cos(r));
    m.set(1, 2, -1.0 * sin(r));

    m.set(2, 1, sin(r));
    m.set(2, 2, cos(r));
    m
}

pub const fn rotation_y(r: f32) -> Matrix<4, 4> {
    let mut m = Matrix::identity();
    m.set(0, 0, cos(r));
    m.set(0, 2, sin(r));

    m.set(2, 0, -1.0 * sin(r));
    m.set(2, 2, cos(r));
    m
}

pub const fn rotation_z(r: f32) -> Matrix<4, 4> {
    let mut m = Matrix::identity();
    m.set(0, 0, cos(r));
    m.set(0, 1, -1.0 * sin(r));

    m.set(1, 0, sin(r));
    m.set(1, 1, cos(r));
    m
}

pub const fn shearing(x_y: f32, x_z: f32, y_x: f32, y_z: f32, z_x: f32, z_y: f32) -> Matrix<4, 4> {
    let mut m = Matrix::identity();
    m.set(0, 1, x_y);
    m.set(0, 2, x_z);
    m.set(1, 0, y_x);
    m.set(1, 2, y_z);
    m.set(2, 0, z_x);
    m.set(2, 1, z_y);
    m
}

pub fn view_transform(from: Point, to: Point, up: Vector) -> Matrix<4, 4> {
    let forwardv = (to - from).normalize();
    let leftv = forwardv.cross(up.normalize());
    let true_up = leftv.cross(forwardv);
    let orientation = Matrix::new([
        [leftv.x(), leftv.y(), leftv.z(), 0.0],
        [true_up.x(), true_up.y(), true_up.z(), 0.0],
        [-forwardv.x(), -forwardv.y(), -forwardv.z(), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    orientation * translation(-from.x(), -from.y(), -from.z())
}

mod tests {
    use super::*;
    #[test]
    fn multiplying_by_a_translation_matrix() {
        const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
        let p = Point {
            x: -3.0,
            y: 4.0,
            z: 5.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 2.0,
                y: 1.0,
                z: 7.0
            }
        );
    }
    #[test]
    fn multiplying_by_the_inverse_of_a_translation_matrix() {
        const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
        let inv = inverse(&TRANSFORM).unwrap();
        let p = Point {
            x: -3.0,
            y: 4.0,
            z: 5.0,
        };
        assert_eq!(
            inv * p,
            Point {
                x: -8.0,
                y: 7.0,
                z: 3.0
            }
        );
    }
    #[test]
    fn translation_does_not_affect_vectors() {
        const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
        let v = Vector {
            x: -3.0,
            y: 4.0,
            z: 5.0,
        };
        assert_eq!(TRANSFORM * v, v);
    }
    #[test]
    fn a_scaling_matrix_applied_to_a_point() {
        const TRANSFORM: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
        let p = Point {
            x: -4.0,
            y: 6.0,
            z: 8.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: -8.0,
                y: 18.0,
                z: 32.0
            }
        );
    }
    #[test]
    fn a_scaling_matrix_applied_to_a_vector() {
        const TRANSFORM: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
        let p = Vector {
            x: -4.0,
            y: 6.0,
            z: 8.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Vector {
                x: -8.0,
                y: 18.0,
                z: 32.0
            }
        );
    }
    #[test]
    fn multiplying_by_the_inverse_of_a_scaling_matrix() {
        const TRANSFORM: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
        let inv = inverse(&TRANSFORM).unwrap();
        let v = Vector {
            x: -4.0,
            y: 6.0,
            z: 8.0,
        };
        assert_eq!(
            inv * v,
            Vector {
                x: -2.0,
                y: 2.0,
                z: 2.0
            }
        );
    }
    #[test]
    fn reflection_is_scaling_by_a_negative_value() {
        const TRANSFORM: Matrix<4, 4> = scaling(-1.0, 1.0, 1.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: -2.0,
                y: 3.0,
                z: 4.0
            }
        );
    }
    #[test]
    fn rotating_a_point_around_the_x_axis() {
        let p = Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        const HALF_QUARTER: Matrix<4, 4> = rotation_x(PI / 4.0);
        const FULL_QUARTER: Matrix<4, 4> = rotation_x(PI / 2.0);
        assert_eq!(
            HALF_QUARTER * p,
            Point {
                x: 0.0,
                y: (2.0_f32).sqrt() / 2.0,
                z: (2.0_f32).sqrt() / 2.0
            }
        );
        assert_eq!(
            FULL_QUARTER * p,
            Point {
                x: 0.0,
                y: 0.0,
                z: 1.0
            }
        );
    }
    #[test]
    fn the_inverse_of_an_x_rotation_rotates_in_the_opposite_direction() {
        let p = Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        const HALF_QUARTER: Matrix<4, 4> = rotation_x(PI / 4.0);
        let inv = inverse(&HALF_QUARTER).unwrap();
        assert_eq!(
            inv * p,
            Point {
                x: 0.0,
                y: (2.0_f32).sqrt() / 2.0,
                z: -1.0 * (2.0_f32).sqrt() / 2.0
            }
        )
    }
    #[test]
    fn rotating_a_point_around_the_y_axis() {
        let p = Point {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        const HALF_QUARTER: Matrix<4, 4> = rotation_y(PI / 4.0);
        const FULL_QUARTER: Matrix<4, 4> = rotation_y(PI / 2.0);
        assert_eq!(
            HALF_QUARTER * p,
            Point {
                x: (2.0_f32).sqrt() / 2.0,
                y: 0.0,
                z: (2.0_f32).sqrt() / 2.0
            }
        );
        assert_eq!(
            FULL_QUARTER * p,
            Point {
                x: 1.0,
                y: 0.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn rotating_a_point_around_the_z_axis() {
        let p = Point {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        const HALF_QUARTER: Matrix<4, 4> = rotation_z(PI / 4.0);
        const FULL_QUARTER: Matrix<4, 4> = rotation_z(PI / 2.0);
        assert_eq!(
            HALF_QUARTER * p,
            Point {
                x: -1.0 * (2.0_f32).sqrt() / 2.0,
                y: (2.0_f32).sqrt() / 2.0,
                z: 0.0
            }
        );
        assert_eq!(
            FULL_QUARTER * p,
            Point {
                x: -1.0,
                y: 0.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn a_shearing_transformation_moves_x_in_proportion_of_y() {
        const TRANSFORM: Matrix<4, 4> = shearing(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 5.0,
                y: 3.0,
                z: 4.0
            }
        );
    }
    #[test]
    fn a_shearing_transformation_moves_x_in_proportion_of_z() {
        const TRANSFORM: Matrix<4, 4> = shearing(0.0, 1.0, 0.0, 0.0, 0.0, 0.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 6.0,
                y: 3.0,
                z: 4.0
            }
        );
    }
    #[test]
    fn a_shearing_transformation_moves_y_in_proportion_of_x() {
        const TRANSFORM: Matrix<4, 4> = shearing(0.0, 0.0, 1.0, 0.0, 0.0, 0.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 2.0,
                y: 5.0,
                z: 4.0
            }
        );
    }
    #[test]
    fn a_shearing_transformation_moves_y_in_proportion_of_z() {
        const TRANSFORM: Matrix<4, 4> = shearing(0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 2.0,
                y: 7.0,
                z: 4.0
            }
        );
    }
    #[test]
    fn a_shearing_transformation_moves_z_in_proportion_of_x() {
        const TRANSFORM: Matrix<4, 4> = shearing(0.0, 0.0, 0.0, 0.0, 1.0, 0.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 2.0,
                y: 3.0,
                z: 6.0
            }
        );
    }
    #[test]
    fn a_shearing_transformation_moves_z_in_proportion_of_y() {
        const TRANSFORM: Matrix<4, 4> = shearing(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let p = Point {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            TRANSFORM * p,
            Point {
                x: 2.0,
                y: 3.0,
                z: 7.0
            }
        );
    }
    #[test]
    fn indivual_transformations_are_applied_in_sequence() {
        let p = Point {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        };
        const A: Matrix<4, 4> = rotation_x(PI / 2.0);
        const B: Matrix<4, 4> = scaling(5.0, 5.0, 5.0);
        const C: Matrix<4, 4> = translation(10.0, 5.0, 7.0);
        let p2 = A * p;
        assert_eq!(
            p2,
            Point {
                x: 1.0,
                y: -1.0,
                z: 0.0
            }
        );
        let p3 = B * p2;
        assert_eq!(
            p3,
            Point {
                x: 5.0,
                y: -5.0,
                z: 0.0
            }
        );
        let p4 = C * p3;
        assert_eq!(
            p4,
            Point {
                x: 15.0,
                y: 0.0,
                z: 7.0
            }
        );
    }
    #[test]
    fn chained_transformations_must_be_applied_in_normal_order() {
        let p = Point {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        };
        const A: Matrix<4, 4> = rotation_x(PI / 2.0);
        const B: Matrix<4, 4> = scaling(5.0, 5.0, 5.0);
        const C: Matrix<4, 4> = translation(10.0, 5.0, 7.0);
        const T: Matrix<4, 4> = A.then(B).then(C);
        assert_eq!(
            T * p,
            Point {
                x: 15.0,
                y: 0.0,
                z: 7.0
            }
        );
    }

    #[test]
    fn fluent_api_transformations_must_be_applied_in_normal_order() {
        let p = Point {
            x: 1.0,
            y: 0.0,
            z: 1.0,
        };
        const T: Matrix<4, 4> = Matrix::identity()
            .then(rotation_x(PI / 2.0))
            .then(scaling(5.0, 5.0, 5.0))
            .then(translation(10.0, 5.0, 7.0));
        assert_eq!(
            T * p,
            Point {
                x: 15.0,
                y: 0.0,
                z: 7.0
            }
        );
    }
}
