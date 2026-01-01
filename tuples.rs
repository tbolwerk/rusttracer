use std::ops::Add;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Neg;
use std::ops::Sub;

pub const EPSILON: f32 = 0.001;

#[derive(Debug, Copy, Clone)]
pub struct Tuple {
    data: [f32; 4],
}

pub fn magnitude(tuple: &Tuple) -> f32 {
    (tuple.x().powi(2) + tuple.y().powi(2) + tuple.z().powi(2) + tuple.w().powi(2)).sqrt()
}

pub fn normalize(tuple: &Tuple) -> Tuple {
    Tuple::new(
        tuple.x() / magnitude(tuple),
        tuple.y() / magnitude(tuple),
        tuple.z() / magnitude(tuple),
        tuple.w() / magnitude(tuple),
    )
}

pub fn dot(a: &Tuple, b: &Tuple) -> f32 {
    a.x() * b.x() + a.y() * b.y() + a.z() * b.z() + a.w() * b.w()
}

pub fn cross(a: &Tuple, b: &Tuple) -> Tuple {
    Tuple::vector(
        a.y() * b.z() - a.z() * b.y(),
        a.z() * b.x() - a.x() * b.z(),
        a.x() * b.y() - a.y() * b.x(),
    )
}

impl Tuple {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { data: [x, y, z, w] }
    }
    pub fn init(value: f32) -> Self {
        Self::new(value, value, value, value)
    }
    pub fn point(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z, 1.0)
    }
    pub fn vector(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z, 0.0)
    }
    pub fn get(&self, index: usize) -> f32 {
        self.data[index]
    }
    pub fn set(&mut self, index: usize, value: f32) {
        self.data[index] = value;
    }
    pub fn x(&self) -> f32 {
        self.get(0)
    }
    pub fn y(&self) -> f32 {
        self.get(1)
    }
    pub fn z(&self) -> f32 {
        self.get(2)
    }
    pub fn w(&self) -> f32 {
        self.get(3)
    }
    pub fn is_vector(&self) -> bool {
        self.w() == 0.0
    }
    pub fn is_point(&self) -> bool {
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
