use crate::matrices::{inverse, Matrix};
use crate::tuples::Tuple;

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

#[test]
fn multiplying_by_a_translation_matrix() {
    const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
    println!("{:#?}", TRANSFORM);
    let p = Tuple::point(-3.0, 4.0, 5.0);
    assert_eq!(TRANSFORM * p, Tuple::point(2.0, 1.0, 7.0));
}
#[test]
fn multiplying_by_the_inverse_of_a_translation_matrix() {
    const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
    let inv = inverse(&TRANSFORM).unwrap();
    let p = Tuple::point(-3.0, 4.0, 5.0);
    assert_eq!(inv * p, Tuple::point(-8.0, 7.0, 3.0));
}
#[test]
fn translation_does_not_affect_vectors() {
    const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
    let v = Tuple::vector(-3.0, 4.0, 5.0);
    assert_eq!(TRANSFORM * v, v);
}
#[test]
fn a_scaling_matrix_applied_to_a_point() {
    const TRANSFORM: Matrix<4, 4> = scaling(2.0, 3.0, 4.0);
    let p = Tuple::point(-4.0, 6.0, 8.0);
    assert_eq!(TRANSFORM * p, Tuple::point(-8.0, 18.0, 32.0));
}
