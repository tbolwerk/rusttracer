pub const EPSILON: f32 = 0.01;

use std::ops::Add;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Neg;
use std::ops::Sub;

pub trait Tuple {
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn z(&self) -> f32;
    fn w(&self) -> f32;
    fn set_x(&mut self, value: f32) -> ();
    fn set_y(&mut self, value: f32) -> ();
    fn set_z(&mut self, value: f32) -> ();
    fn is_point(&self) -> bool {
        self.w() == 1.0
    }
    fn is_vector(&self) -> bool {
        self.w() == 0.0
    }
    fn get(&self, index: usize) -> f32 {
        match index {
            0 => self.x(),
            1 => self.y(),
            2 => self.z(),
            3 => self.w(),
            _ => panic!("Index out of bound {index}"),
        }
    }
    fn set(&mut self, index: usize, value: f32) -> () {
        match index {
            0 => self.set_x(value),
            1 => self.set_y(value),
            2 => self.set_z(value),
            _ => (),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}
impl Tuple for Vector {
    fn x(&self) -> f32 {
        self.x
    }
    fn y(&self) -> f32 {
        self.y
    }
    fn z(&self) -> f32 {
        self.z
    }
    fn w(&self) -> f32 {
        0.0
    }
    fn set_x(&mut self, value: f32) -> () {
        self.x = value;
    }
    fn set_y(&mut self, value: f32) -> () {
        self.y = value
    }
    fn set_z(&mut self, value: f32) -> () {
        self.z = value;
    }
}
impl Tuple for Point {
    fn x(&self) -> f32 {
        self.x
    }
    fn y(&self) -> f32 {
        self.y
    }
    fn z(&self) -> f32 {
        self.z
    }
    fn w(&self) -> f32 {
        1.0
    }
    fn set_x(&mut self, value: f32) -> () {
        self.x = value;
    }
    fn set_y(&mut self, value: f32) -> () {
        self.y = value;
    }
    fn set_z(&mut self, value: f32) -> () {
        self.z = value;
    }
}
impl Add<Vector> for Point {
    type Output = Point;
    fn add(self, rhs: Vector) -> Self::Output {
        Point {
            x: self.x() + rhs.x(),
            y: self.y() + rhs.y(),
            z: self.z() + rhs.z(),
        }
    }
}
impl Add<Vector> for Vector {
    type Output = Vector;
    fn add(self, rhs: Vector) -> Self::Output {
        Vector {
            x: self.x() + rhs.x(),
            y: self.y() + rhs.y(),
            z: self.z() + rhs.z(),
        }
    }
}
impl Sub<Point> for Point {
    type Output = Vector;
    fn sub(self, rhs: Point) -> Self::Output {
        Vector {
            x: self.x() - rhs.x(),
            y: self.y() - rhs.y(),
            z: self.z() - rhs.z(),
        }
    }
}
impl Sub<Vector> for Point {
    type Output = Point;
    fn sub(self, rhs: Vector) -> Self::Output {
        Point {
            x: self.x() - rhs.x(),
            y: self.y() - rhs.y(),
            z: self.z() - rhs.z(),
        }
    }
}
impl Sub<Vector> for Vector {
    type Output = Vector;
    fn sub(self, rhs: Vector) -> Self::Output {
        Vector {
            x: self.x() - rhs.x(),
            y: self.y() - rhs.y(),
            z: self.z() - rhs.z(),
        }
    }
}
impl Neg for Point {
    type Output = Point;
    fn neg(self) -> Self::Output {
        Point {
            x: -self.x(),
            y: -self.y(),
            z: -self.z(),
        }
    }
}
impl Neg for Vector {
    type Output = Vector;
    fn neg(self) -> Self::Output {
        Vector {
            x: -self.x(),
            y: -self.y(),
            z: -self.z(),
        }
    }
}

impl Mul<f32> for Point {
    type Output = Point;
    fn mul(self, rhs: f32) -> Self::Output {
        Point {
            x: self.x() * rhs,
            y: self.y() * rhs,
            z: self.z() * rhs,
        }
    }
}
impl Mul<f32> for Vector {
    type Output = Vector;
    fn mul(self, rhs: f32) -> Self::Output {
        Vector {
            x: self.x() * rhs,
            y: self.y() * rhs,
            z: self.z() * rhs,
        }
    }
}
impl Mul<f32> for Color {
    type Output = Color;
    fn mul(self, rhs: f32) -> Self::Output {
        Color {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
        }
    }
}
impl Mul<Color> for Color {
    type Output = Color;
    fn mul(self, rhs: Color) -> Self::Output {
        Color {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
        }
    }
}
impl Add<Color> for Color {
    type Output = Color;
    fn add(self, rhs: Color) -> Self::Output {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}
impl Div<f32> for Point {
    type Output = Point;
    fn div(self, rhs: f32) -> Self::Output {
        Point {
            x: self.x() / rhs,
            y: self.y() / rhs,
            z: self.z() / rhs,
        }
    }
}
impl Div<f32> for Vector {
    type Output = Vector;
    fn div(self, rhs: f32) -> Self::Output {
        Vector {
            x: self.x() / rhs,
            y: self.y() / rhs,
            z: self.z() / rhs,
        }
    }
}

impl Vector {
    pub fn magnitude(self) -> f32 {
        (self.x().powi(2) + self.y().powi(2) + self.z().powi(2)).sqrt()
    }
    pub fn normalize(self) -> Vector {
        Vector {
            x: self.x() / self.magnitude(),
            y: self.y() / self.magnitude(),
            z: self.z() / self.magnitude(),
        }
    }
    pub fn dot(self, other: Vector) -> f32 {
        self.x() * other.x() + self.y() * other.y() + self.z() * other.z()
    }
    pub fn cross(self, other: Vector) -> Vector {
        Vector {
            x: self.y() * other.z() - self.z() * other.y(),
            y: self.z() * other.x() - self.x() * other.z(),
            z: self.x() * other.y() - self.y() * other.x(),
        }
    }
    pub fn reflect(self, normal: Vector) -> Vector {
        self - (normal * (2.0_f32 * self.dot(normal)))
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        (self.x() - other.x()).abs() <= EPSILON
            && (self.y() - other.y()).abs() <= EPSILON
            && (self.z() - other.z()).abs() <= EPSILON
            && (self.w() - other.w()).abs() <= EPSILON
    }
}
impl PartialEq for Vector {
    fn eq(&self, other: &Self) -> bool {
        (self.x() - other.x()).abs() <= EPSILON
            && (self.y() - other.y()).abs() <= EPSILON
            && (self.z() - other.z()).abs() <= EPSILON
            && (self.w() - other.w()).abs() <= EPSILON
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        (self.r - other.r).abs() <= EPSILON
            && (self.g - other.g).abs() <= EPSILON
            && (self.b - other.b).abs() <= EPSILON
    }
}
impl Default for Point {
    fn default() -> Self {
        Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Default for Vector {
    fn default() -> Self {
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}
mod tests {
    use super::*;
    #[test]
    fn a_tuple_with_w_1_is_a_point() {
        let tuple = Point {
            x: 4.3,
            y: -4.2,
            z: 3.1,
        };
        assert_eq!(tuple.is_point(), true);
        assert_eq!(tuple.is_vector(), false);
    }
    #[test]
    fn a_tuple_with_w_0_is_a_vector() {
        let tuple = Vector {
            x: 4.3,
            y: -4.2,
            z: 3.1,
        };
        assert_eq!(tuple.is_point(), false);
        assert_eq!(tuple.is_vector(), true);
    }
    #[test]
    fn creates_tuples_with_w_1() {
        let tuple = Point {
            x: 4.3,
            y: -4.2,
            z: 3.1,
        };
        assert_eq!(
            Point {
                x: 4.3,
                y: -4.2,
                z: 3.1
            },
            tuple
        );
    }
    #[test]
    fn creates_vector_with_w_0() {
        let tuple = Vector {
            x: 4.3,
            y: -4.2,
            z: 3.1,
        };
        assert_eq!(
            Vector {
                x: 4.3,
                y: -4.2,
                z: 3.1
            },
            tuple
        );
    }
    #[test]
    fn adding_two_tuples() {
        let a1 = Point {
            x: 3.0,
            y: -2.0,
            z: 5.0,
        };
        let a2 = Vector {
            x: -2.0,
            y: 3.0,
            z: 1.0,
        };
        assert_eq!(
            a1 + a2,
            Point {
                x: 1.0,
                y: 1.0,
                z: 6.0
            }
        );
    }
    #[test]
    fn subtracting_two_points() {
        let p1 = Point {
            x: 3.0,
            y: 2.0,
            z: 1.0,
        };
        let p2 = Point {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        };
        assert_eq!(
            p1 - p2,
            Vector {
                x: -2.0,
                y: -4.0,
                z: -6.0
            }
        );
    }
    #[test]
    fn subtracting_a_vector_from_a_point() {
        let p = Point {
            x: 3.0,
            y: 2.0,
            z: 1.0,
        };
        let v = Vector {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        };
        assert_eq!(
            p - v,
            Point {
                x: -2.0,
                y: -4.0,
                z: -6.0
            }
        );
    }
    #[test]
    fn subtracting_two_vectors() {
        let v1 = Vector {
            x: 3.0,
            y: 2.0,
            z: 1.0,
        };
        let v2 = Vector {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        };
        assert_eq!(
            v1 - v2,
            Vector {
                x: -2.0,
                y: -4.0,
                z: -6.0
            }
        );
    }
    #[test]
    fn subtracting_a_vector_from_the_zero_vector() {
        let zero = Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let v = Vector {
            x: 1.0,
            y: -2.0,
            z: 3.0,
        };
        assert_eq!(
            zero - v,
            Vector {
                x: -1.0,
                y: 2.0,
                z: -3.0
            }
        );
    }

    fn construct_point_and_vector(x: f32, y: f32, z: f32) -> (Point, Vector) {
        let a = Point { x, y, z };
        let b = Vector { x, y, z };
        (a, b)
    }

    #[test]
    fn negating_a_tuple() {
        let (a, b) = construct_point_and_vector(1.0, -2.0, 3.0);
        assert_eq!(
            -a,
            Point {
                x: -1.0,
                y: 2.0,
                z: -3.0
            }
        );
        assert_eq!(
            -b,
            Vector {
                x: -1.0,
                y: 2.0,
                z: -3.0
            }
        );
    }
    #[test]
    fn multiplying_a_tuple_by_a_scalar() {
        let (a, b) = construct_point_and_vector(1.0, -2.0, 3.0);
        assert_eq!(
            a * 3.5,
            Point {
                x: 3.5,
                y: -7.0,
                z: 10.5
            }
        );
        assert_eq!(
            b * 3.5,
            Vector {
                x: 3.5,
                y: -7.0,
                z: 10.5
            }
        );
    }
    #[test]
    fn multiplying_a_tuple_by_a_fraction() {
        let (a, b) = construct_point_and_vector(1.0, -2.0, 3.0);
        assert_eq!(
            a * 0.5,
            Point {
                x: 0.5,
                y: -1.0,
                z: 1.5
            }
        );
        assert_eq!(
            b * 0.5,
            Vector {
                x: 0.5,
                y: -1.0,
                z: 1.5
            }
        );
    }
    #[test]
    fn dividing_a_tuple_by_a_scalar() {
        let (a, b) = construct_point_and_vector(1.0, -2.0, 3.0);
        assert_eq!(
            a / 2.0,
            Point {
                x: 0.5,
                y: -1.0,
                z: 1.5
            }
        );
        assert_eq!(
            b / 2.0,
            Vector {
                x: 0.5,
                y: -1.0,
                z: 1.5
            }
        );
    }
    #[test]
    fn computing_the_magnitude_of_vector_1_0_0() {
        let v = Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(v.magnitude(), 1.0);
    }
    #[test]
    fn computing_the_magnitude_of_vector_0_1_0() {
        let v = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        assert_eq!(v.magnitude(), 1.0);
    }
    #[test]
    fn computing_the_magnitude_of_vector_0_0_1() {
        let v = Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        assert_eq!(v.magnitude(), 1.0);
    }
    #[test]
    fn computing_the_magnitude_of_vector_1_2_3() {
        let v = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        assert_eq!(v.magnitude(), (14.0_f32).sqrt());
    }
    #[test]
    fn computing_the_magnitude_of_vector_neg_1_2_3() {
        let v = Vector {
            x: -1.0,
            y: -2.0,
            z: -3.0,
        };
        assert_eq!(v.magnitude(), (14.0_f32).sqrt());
    }
    #[test]
    fn normalizing_vector_4_0_0_gives_1_0_0() {
        let v = Vector {
            x: 4.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(
            v.normalize(),
            Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn normalizing_vector_1_2_3() {
        let v = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        assert_eq!(
            v.normalize(),
            Vector {
                x: 0.26726,
                y: 0.53452,
                z: 0.80178
            }
        );
    }
    #[test]
    fn the_magnitude_of_a_normalized_vector() {
        let v = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let norm = v.normalize();
        assert_eq!(1.0 - norm.magnitude().abs() <= EPSILON, true);
    }
    #[test]
    fn the_dot_product_of_two_tuples() {
        let a = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let b = Vector {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(a.dot(b), 20.0);
    }
    #[test]
    fn the_cross_product_of_two_vectors() {
        let a = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let b = Vector {
            x: 2.0,
            y: 3.0,
            z: 4.0,
        };
        assert_eq!(
            a.cross(b),
            Vector {
                x: -1.0,
                y: 2.0,
                z: -1.0
            }
        );
        assert_eq!(
            b.cross(a),
            Vector {
                x: 1.0,
                y: -2.0,
                z: 1.0
            }
        );
    }
    #[test]
    fn reflecting_a_vector_approaching_at_45_degree() {
        let v = Vector {
            x: 1.0,
            y: -1.0,
            z: 0.0,
        };
        let n = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let r = v.reflect(n);
        assert_eq!(
            r,
            Vector {
                x: 1.0,
                y: 1.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn reflecting_a_vector_off_a_slanted_surface() {
        let v = Vector {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        };
        let n = Vector {
            x: 2.0_f32.sqrt() / 2.0,
            y: 2.0_f32.sqrt() / 2.0,
            z: 0.0,
        };
        let r = v.reflect(n);
        assert_eq!(
            r,
            Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0
            }
        );
    }
}
