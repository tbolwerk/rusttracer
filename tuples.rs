use std::ops::Add;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Neg;
use std::ops::Sub;

pub const EPSILON: f32 = 0.001;

pub mod mytuples {
    use crate::tuples::EPSILON;
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
    #[derive(Debug, Clone)]
    pub struct Vector {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }
    #[derive(Debug, Clone)]
    pub struct Point {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }
    #[derive(Debug, Clone)]
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
        pub fn magnitude(&self) -> f32 {
            (self.x().powi(2) + self.y().powi(2) + self.z().powi(2)).sqrt()
        }
        pub fn normalize(&self) -> Vector {
            Vector {
                x: self.x() / self.magnitude(),
                y: self.y() / self.magnitude(),
                z: self.z() / self.magnitude(),
            }
        }
        pub fn dot(&self, other: &Vector) -> f32 {
            self.x() * other.x() + self.y() * other.y() + self.z() * other.z()
        }
        pub fn cross(&self, other: &Vector) -> Vector {
            Vector {
                x: self.y() * other.z() - self.z() * other.y(),
                y: self.z() * other.x() - self.x() * other.z(),
                z: self.x() * other.y() - self.y() * other.x(),
            }
        }
        pub fn reflect(&self, normal: &Vector) -> Vector {
            self.clone() - (normal.clone() * (2.0_f32 * self.dot(&normal)))
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
        assert_eq!(a.dot(&b), 20.0);
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
            a.cross(&b),
            Vector {
                x: -1.0,
                y: 2.0,
                z: -1.0
            }
        );
        assert_eq!(
            b.cross(&a),
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
        let r = v.reflect(&n);
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
        let r = v.reflect(&n);
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

#[derive(Debug, Copy, Clone)]
pub struct Tuple {
    data: [f32; 4],
}
pub fn magnitude(tuple: &Tuple) -> f32 {
    (tuple.x().powi(2) + tuple.y().powi(2) + tuple.z().powi(2)).sqrt()
}

pub fn normalize(tuple: &Tuple) -> Tuple {
    Tuple::vector(
        tuple.x() / magnitude(tuple),
        tuple.y() / magnitude(tuple),
        tuple.z() / magnitude(tuple),
    )
}

pub const fn dot(a: &Tuple, b: &Tuple) -> f32 {
    a.x() * b.x() + a.y() * b.y() + a.z() * b.z()
}

pub const fn cross(a: &Tuple, b: &Tuple) -> Tuple {
    Tuple::vector(
        a.y() * b.z() - a.z() * b.y(),
        a.z() * b.x() - a.x() * b.z(),
        a.x() * b.y() - a.y() * b.x(),
    )
}

pub fn reflect(a: &Tuple, normal: &Tuple) -> Tuple {
    a.clone() - (normal.clone() * (2.0_f32 * dot(a, normal)))
}

impl Tuple {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { data: [x, y, z, w] }
    }
    pub const fn init(value: f32) -> Self {
        Self::new(value, value, value, value)
    }
    pub const fn point(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z, 1.0)
    }
    pub const fn vector(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z, 0.0)
    }
    pub const fn get(&self, index: usize) -> f32 {
        self.data[index]
    }
    pub const fn set(&mut self, index: usize, value: f32) {
        self.data[index] = value;
    }
    pub const fn x(&self) -> f32 {
        self.get(0)
    }
    pub const fn y(&self) -> f32 {
        self.get(1)
    }
    pub const fn z(&self) -> f32 {
        self.get(2)
    }
    pub const fn w(&self) -> f32 {
        self.get(3)
    }
    pub const fn is_vector(&self) -> bool {
        self.w() == 0.0
    }
    pub const fn is_point(&self) -> bool {
        self.w() == 1.0
    }
}
impl PartialEq for Tuple {
    fn eq(&self, other: &Self) -> bool {
        (self.x() - other.x()).abs() <= EPSILON
            && (self.y() - other.y()).abs() <= EPSILON
            && (self.z() - other.z()).abs() <= EPSILON
            && (self.w() - other.w()).abs() <= EPSILON
    }
}
impl Add for Tuple {
    type Output = Tuple;
    fn add(self, other: Tuple) -> Self::Output {
        Tuple::new(
            self.x() + other.x(),
            self.y() + other.y(),
            self.z() + other.z(),
            self.w() + other.w(),
        )
    }
}
impl Sub for Tuple {
    type Output = Tuple;
    fn sub(self, other: Tuple) -> Self::Output {
        Tuple::new(
            self.x() - other.x(),
            self.y() - other.y(),
            self.z() - other.z(),
            self.w() - other.w(),
        )
    }
}
impl Neg for Tuple {
    type Output = Tuple;
    fn neg(self) -> Self::Output {
        Tuple::new(
            self.x() * -1.0,
            self.y() * -1.0,
            self.z() * -1.0,
            self.w() * -1.0,
        )
    }
}
impl Mul<f32> for Tuple {
    type Output = Tuple;
    fn mul(self, rhs: f32) -> Self::Output {
        Tuple::new(
            self.x() * rhs,
            self.y() * rhs,
            self.z() * rhs,
            self.w() * rhs,
        )
    }
}
impl Mul<Tuple> for Tuple {
    type Output = Tuple;
    fn mul(self, rhs: Tuple) -> Self::Output {
        Tuple::new(
            self.x() * rhs.x(),
            self.y() * rhs.y(),
            self.z() * rhs.z(),
            self.w() * rhs.w(),
        )
    }
}
impl Div<f32> for Tuple {
    type Output = Tuple;
    fn div(self, rhs: f32) -> Self::Output {
        Tuple::new(
            self.x() / rhs,
            self.y() / rhs,
            self.z() / rhs,
            self.w() / rhs,
        )
    }
}

#[test]
fn a_tuple_with_w_1_is_a_point() {
    let tuple = Tuple::new(4.3, -4.2, 3.1, 1.0);
    assert_eq!(tuple.is_point(), true);
    assert_eq!(tuple.is_vector(), false);
}
#[test]
fn a_tuple_with_w_0_is_a_vector() {
    let tuple = Tuple::new(4.3, -4.2, 3.1, 0.0);
    assert_eq!(tuple.is_point(), false);
    assert_eq!(tuple.is_vector(), true);
}
#[test]
fn creates_tuples_with_w_1() {
    let tuple = Tuple::point(4.3, -4.2, 3.1);
    assert_eq!(Tuple::new(4.3, -4.2, 3.1, 1.0), tuple);
}
#[test]
fn creates_vector_with_w_0() {
    let tuple = Tuple::vector(4.3, -4.2, 3.1);
    assert_eq!(Tuple::new(4.3, -4.2, 3.1, 0.0), tuple);
}
#[test]
fn adding_two_tuples() {
    let a1 = Tuple::new(3.0, -2.0, 5.0, 1.0);
    let a2 = Tuple::new(-2.0, 3.0, 1.0, 0.0);
    assert_eq!(a1 + a2, Tuple::new(1.0, 1.0, 6.0, 1.0));
}
#[test]
fn subtracting_two_points() {
    let p1 = Tuple::point(3.0, 2.0, 1.0);
    let p2 = Tuple::point(5.0, 6.0, 7.0);
    assert_eq!(p1 - p2, Tuple::vector(-2.0, -4.0, -6.0));
}
#[test]
fn subtracting_a_vector_from_a_point() {
    let p = Tuple::point(3.0, 2.0, 1.0);
    let v = Tuple::vector(5.0, 6.0, 7.0);
    assert_eq!(p - v, Tuple::point(-2.0, -4.0, -6.0));
}
#[test]
fn subtracting_two_vectors() {
    let v1 = Tuple::vector(3.0, 2.0, 1.0);
    let v2 = Tuple::vector(5.0, 6.0, 7.0);
    assert_eq!(v1 - v2, Tuple::vector(-2.0, -4.0, -6.0));
}
#[test]
fn subtracting_a_vector_from_the_zero_vector() {
    let zero = Tuple::vector(0.0, 0.0, 0.0);
    let v = Tuple::vector(1.0, -2.0, 3.0);
    assert_eq!(zero - v, Tuple::vector(-1.0, 2.0, -3.0));
}
#[test]
fn negating_a_tuple() {
    let a = Tuple::new(1.0, -2.0, 3.0, -4.0);
    assert_eq!(-a, Tuple::new(-1.0, 2.0, -3.0, 4.0));
}
#[test]
fn multiplying_a_tuple_by_a_scalar() {
    let a = Tuple::new(1.0, -2.0, 3.0, -4.0);
    assert_eq!(a * 3.5, Tuple::new(3.5, -7.0, 10.5, -14.0));
}
#[test]
fn multiplying_a_tuple_by_a_fraction() {
    let a = Tuple::new(1.0, -2.0, 3.0, -4.0);
    assert_eq!(a * 0.5, Tuple::new(0.5, -1.0, 1.5, -2.0));
}
#[test]
fn dividing_a_tuple_by_a_scalar() {
    let a = Tuple::new(1.0, -2.0, 3.0, -4.0);
    assert_eq!(a / 2.0, Tuple::new(0.5, -1.0, 1.5, -2.0));
}
#[test]
fn computing_the_magnitude_of_vector_1_0_0() {
    let v = Tuple::vector(1.0, 0.0, 0.0);
    assert_eq!(magnitude(&v), 1.0);
}
#[test]
fn computing_the_magnitude_of_vector_0_1_0() {
    let v = Tuple::vector(0.0, 1.0, 0.0);
    assert_eq!(magnitude(&v), 1.0);
}
#[test]
fn computing_the_magnitude_of_vector_0_0_1() {
    let v = Tuple::vector(0.0, 0.0, 1.0);
    assert_eq!(magnitude(&v), 1.0);
}
#[test]
fn computing_the_magnitude_of_vector_1_2_3() {
    let v = Tuple::vector(1.0, 2.0, 3.0);
    assert_eq!(magnitude(&v), (14.0_f32).sqrt());
}
#[test]
fn computing_the_magnitude_of_vector_neg_1_2_3() {
    let v = Tuple::vector(-1.0, -2.0, -3.0);
    assert_eq!(magnitude(&v), (14.0_f32).sqrt());
}
#[test]
fn normalizing_vector_4_0_0_gives_1_0_0() {
    let v = Tuple::vector(4.0, 0.0, 0.0);
    assert_eq!(normalize(&v), Tuple::vector(1.0, 0.0, 0.0));
}
#[test]
fn normalizing_vector_1_2_3() {
    let v = Tuple::vector(1.0, 2.0, 3.0);
    assert_eq!(normalize(&v), Tuple::vector(0.26726, 0.53452, 0.80178));
}
#[test]
fn the_magnitude_of_a_normalized_vector() {
    let v = Tuple::vector(1.0, 2.0, 3.0);
    let norm = normalize(&v);
    assert_eq!((1.0 - magnitude(&norm)).abs() <= EPSILON, true);
}
#[test]
fn the_dot_product_of_two_tuples() {
    let a = Tuple::vector(1.0, 2.0, 3.0);
    let b = Tuple::vector(2.0, 3.0, 4.0);
    assert_eq!(dot(&a, &b), 20.0);
}
#[test]
fn the_cross_product_of_two_vectors() {
    let a = Tuple::vector(1.0, 2.0, 3.0);
    let b = Tuple::vector(2.0, 3.0, 4.0);
    assert_eq!(cross(&a, &b), Tuple::vector(-1.0, 2.0, -1.0));
    assert_eq!(cross(&b, &a), Tuple::vector(1.0, -2.0, 1.0));
}
#[test]
fn reflecting_a_vector_approaching_at_45_degree() {
    let v = Tuple::vector(1.0, -1.0, 0.0);
    let n = Tuple::vector(0.0, 1.0, 0.0);
    let r = reflect(&v, &n);
    assert_eq!(r, Tuple::vector(1.0, 1.0, 0.0));
}
#[test]
fn reflecting_a_vector_off_a_slanted_surface() {
    let v = Tuple::vector(0.0, -1.0, 0.0);
    let n = Tuple::vector(2.0_f32.sqrt() / 2.0, 2.0_f32.sqrt() / 2.0, 0.0);
    let r = reflect(&v, &n);
    assert_eq!(r, Tuple::vector(1.0, 0.0, 0.0));
}

pub mod external_tuples {
    use crate::tuples::*;
    #[derive(Debug, Copy, Clone)]
    pub enum TupleKind {
        Color(Tuple),
        Vector(Tuple),
        Point(Tuple),
    }

    impl TupleKind {
        pub const fn color(r: f32, g: f32, b: f32) -> TupleKind {
            TupleKind::Color(Tuple::new(r, g, b, 1.0))
        }
        pub const fn vector(x: f32, y: f32, z: f32) -> TupleKind {
            TupleKind::Vector(Tuple::vector(x, y, z))
        }
        pub const fn point(x: f32, y: f32, z: f32) -> TupleKind {
            TupleKind::Point(Tuple::point(x, y, z))
        }
        pub const fn unwrap(self) -> Tuple {
            match self {
                TupleKind::Vector(v) => v,
                TupleKind::Color(c) => c,
                TupleKind::Point(p) => p,
            }
        }
        pub const fn wrap(t: Tuple) -> TupleKind {
            if t.is_point() {
                return TupleKind::Point(t);
            }
            if t.is_vector() {
                return TupleKind::Vector(t);
            }
            TupleKind::Color(t)
        }
        pub const fn x(&self) -> f32 {
            self.unwrap().x()
        }
        pub const fn y(&self) -> f32 {
            self.unwrap().y()
        }
        pub const fn z(&self) -> f32 {
            self.unwrap().z()
        }
        pub const fn w(&self) -> f32 {
            self.unwrap().w()
        }
        pub const fn get(&self, index: usize) -> f32 {
            self.unwrap().get(index)
        }
        pub const fn set(&mut self, index: usize, value: f32) {
            self.unwrap().set(index, value);
        }
        pub const fn is_point(&self) -> bool {
            matches!(self, TupleKind::Point(_))
        }
        pub const fn is_vector(&self) -> bool {
            matches!(self, TupleKind::Vector(_))
        }
    }

    pub trait VectorMath {
        fn magnitude(&self) -> f32;
        fn normalize(&self) -> TupleKind;
        fn dot(&self, b: &TupleKind) -> f32;
        fn cross(&self, b: &TupleKind) -> TupleKind;
        fn reflect(&self, normal: &TupleKind) -> TupleKind;
    }

    impl VectorMath for TupleKind {
        fn magnitude(&self) -> f32 {
            magnitude(&self.unwrap())
        }
        fn normalize(&self) -> TupleKind {
            TupleKind::wrap(normalize(&self.unwrap()))
        }
        fn dot(&self, b: &TupleKind) -> f32 {
            dot(&self.unwrap(), &b.unwrap())
        }
        fn cross(&self, b: &TupleKind) -> TupleKind {
            TupleKind::wrap(cross(&self.unwrap(), &b.unwrap()))
        }
        fn reflect(&self, normal: &TupleKind) -> TupleKind {
            TupleKind::wrap(reflect(&self.unwrap(), &normal.unwrap()))
        }
    }

    impl PartialEq for TupleKind {
        fn eq(&self, other: &Self) -> bool {
            self.unwrap() == other.unwrap()
        }
    }

    impl Add for TupleKind {
        type Output = TupleKind;
        fn add(self, rhs: TupleKind) -> Self::Output {
            let result = self.unwrap() + rhs.unwrap();
            TupleKind::wrap(result)
        }
    }

    impl Sub for TupleKind {
        type Output = TupleKind;
        fn sub(self, rhs: TupleKind) -> Self::Output {
            let result = self.unwrap() - rhs.unwrap();
            TupleKind::wrap(result)
        }
    }

    impl Neg for TupleKind {
        type Output = TupleKind;
        fn neg(self) -> Self::Output {
            let result = -self.unwrap();
            TupleKind::wrap(result)
        }
    }

    impl Mul<f32> for TupleKind {
        type Output = TupleKind;
        fn mul(self, rhs: f32) -> Self::Output {
            let result = self.unwrap() * rhs;
            TupleKind::wrap(result)
        }
    }

    impl Mul<TupleKind> for TupleKind {
        type Output = TupleKind;
        fn mul(self, rhs: TupleKind) -> Self::Output {
            let result = self.unwrap() * rhs.unwrap();
            TupleKind::wrap(result)
        }
    }

    impl Div<f32> for TupleKind {
        type Output = TupleKind;
        fn div(self, rhs: f32) -> Self::Output {
            let result = self.unwrap() / rhs;
            TupleKind::wrap(result)
        }
    }
    #[test]
    fn a_tuple_with_w_1_is_a_point() {
        let tuple = TupleKind::wrap(Tuple::new(4.3, -4.2, 3.1, 1.0));
        assert_eq!(tuple.is_point(), true);
        assert_eq!(tuple.is_vector(), false);
    }
    #[test]
    fn a_tuple_with_w_0_is_a_vector() {
        let tuple = TupleKind::wrap(Tuple::new(4.3, -4.2, 3.1, 0.0));
        assert_eq!(tuple.is_point(), false);
        assert_eq!(tuple.is_vector(), true);
    }
    #[test]
    fn creates_tuples_with_w_1() {
        let tuple = TupleKind::wrap(Tuple::point(4.3, -4.2, 3.1));
        assert_eq!(Tuple::new(4.3, -4.2, 3.1, 1.0), tuple.unwrap());
    }
    #[test]
    fn creates_vector_with_w_0() {
        let tuple = TupleKind::vector(4.3, -4.2, 3.1);
        assert_eq!(Tuple::new(4.3, -4.2, 3.1, 0.0), tuple.unwrap());
    }
    #[test]
    fn adding_two_tuples() {
        let a1 = TupleKind::wrap(Tuple::new(3.0, -2.0, 5.0, 1.0));
        let a2 = TupleKind::wrap(Tuple::new(-2.0, 3.0, 1.0, 0.0));
        assert_eq!(a1 + a2, TupleKind::wrap(Tuple::new(1.0, 1.0, 6.0, 1.0)));
    }
    #[test]
    fn subtracting_two_points() {
        let p1 = TupleKind::wrap(Tuple::point(3.0, 2.0, 1.0));
        let p2 = TupleKind::wrap(Tuple::point(5.0, 6.0, 7.0));
        assert_eq!(p1 - p2, TupleKind::vector(-2.0, -4.0, -6.0));
    }
    #[test]
    fn subtracting_a_vector_from_a_point() {
        let p = TupleKind::wrap(Tuple::point(3.0, 2.0, 1.0));
        let v = TupleKind::vector(5.0, 6.0, 7.0);
        assert_eq!(p - v, TupleKind::wrap(Tuple::point(-2.0, -4.0, -6.0)));
    }
    #[test]
    fn subtracting_two_vectors() {
        let v1 = TupleKind::vector(3.0, 2.0, 1.0);
        let v2 = TupleKind::vector(5.0, 6.0, 7.0);
        assert_eq!(v1 - v2, TupleKind::vector(-2.0, -4.0, -6.0));
    }
    #[test]
    fn subtracting_a_vector_from_the_zero_vector() {
        let zero = TupleKind::vector(0.0, 0.0, 0.0);
        let v = TupleKind::vector(1.0, -2.0, 3.0);
        assert_eq!(zero - v, TupleKind::vector(-1.0, 2.0, -3.0));
    }
    #[test]
    fn negating_a_tuple() {
        let a = TupleKind::wrap(Tuple::new(1.0, -2.0, 3.0, -4.0));
        assert_eq!(-a, TupleKind::wrap(Tuple::new(-1.0, 2.0, -3.0, 4.0)));
    }
    #[test]
    fn multiplying_a_tuple_by_a_scalar() {
        let a = TupleKind::wrap(Tuple::new(1.0, -2.0, 3.0, -4.0));
        assert_eq!(a * 3.5, TupleKind::wrap(Tuple::new(3.5, -7.0, 10.5, -14.0)));
    }
    #[test]
    fn multiplying_a_tuple_by_a_fraction() {
        let a = TupleKind::wrap(Tuple::new(1.0, -2.0, 3.0, -4.0));
        assert_eq!(a * 0.5, TupleKind::wrap(Tuple::new(0.5, -1.0, 1.5, -2.0)));
    }
    #[test]
    fn dividing_a_tuple_by_a_scalar() {
        let a = TupleKind::wrap(Tuple::new(1.0, -2.0, 3.0, -4.0));
        assert_eq!(a / 2.0, TupleKind::wrap(Tuple::new(0.5, -1.0, 1.5, -2.0)));
    }
    #[test]
    fn computing_the_magnitude_of_vector_1_0_0() {
        let v = TupleKind::vector(1.0, 0.0, 0.0);
        assert_eq!(v.magnitude(), 1.0);
    }
    #[test]
    fn computing_the_magnitude_of_vector_0_1_0() {
        let v = TupleKind::vector(0.0, 1.0, 0.0);
        assert_eq!(v.magnitude(), 1.0);
    }
    #[test]
    fn computing_the_magnitude_of_vector_0_0_1() {
        let v = TupleKind::vector(0.0, 0.0, 1.0);
        assert_eq!(v.magnitude(), 1.0);
    }
    #[test]
    fn computing_the_magnitude_of_vector_1_2_3() {
        let v = TupleKind::vector(1.0, 2.0, 3.0);
        assert_eq!(v.magnitude(), (14.0_f32).sqrt());
    }
    #[test]
    fn computing_the_magnitude_of_vector_neg_1_2_3() {
        let v = TupleKind::vector(-1.0, -2.0, -3.0);
        assert_eq!(v.magnitude(), (14.0_f32).sqrt());
    }
    #[test]
    fn normalizing_vector_4_0_0_gives_1_0_0() {
        let v = TupleKind::vector(4.0, 0.0, 0.0);
        assert_eq!(v.normalize(), TupleKind::vector(1.0, 0.0, 0.0));
    }
    #[test]
    fn normalizing_vector_1_2_3() {
        let v = TupleKind::vector(1.0, 2.0, 3.0);
        assert_eq!(v.normalize(), TupleKind::vector(0.26726, 0.53452, 0.80178));
    }
    #[test]
    fn the_magnitude_of_a_normalized_vector() {
        let v = TupleKind::vector(1.0, 2.0, 3.0);
        let norm = v.normalize();
        assert_eq!((1.0 - norm.magnitude()).abs() <= EPSILON, true);
    }
    #[test]
    fn the_dot_product_of_two_tuples() {
        let a = TupleKind::vector(1.0, 2.0, 3.0);
        let b = TupleKind::vector(2.0, 3.0, 4.0);
        assert_eq!(a.dot(&b), 20.0);
    }
    #[test]
    fn the_cross_product_of_two_vectors() {
        let a = TupleKind::vector(1.0, 2.0, 3.0);
        let b = TupleKind::vector(2.0, 3.0, 4.0);
        assert_eq!(a.cross(&b), TupleKind::vector(-1.0, 2.0, -1.0));
        assert_eq!(b.cross(&a), TupleKind::vector(1.0, -2.0, 1.0));
    }
    #[test]
    fn reflecting_a_vector_approaching_at_45_degree() {
        let v = TupleKind::vector(1.0, -1.0, 0.0);
        let n = TupleKind::vector(0.0, 1.0, 0.0);
        let r = v.reflect(&n);
        assert_eq!(r, TupleKind::vector(1.0, 1.0, 0.0));
    }
    #[test]
    fn reflecting_a_vector_off_a_slanted_surface() {
        let v = TupleKind::vector(0.0, -1.0, 0.0);
        let n = TupleKind::vector(2.0_f32.sqrt() / 2.0, 2.0_f32.sqrt() / 2.0, 0.0);
        let r = v.reflect(&n);
        assert_eq!(r, TupleKind::vector(1.0, 0.0, 0.0));
    }
}
