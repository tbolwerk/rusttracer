use crate::matrices::{inverse, Matrix};
use crate::tuples::Tuple;

const PI: f32 = 3.14;

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

const fn cos(x: f32) -> f32 {
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;
    1.0 - x2 / 2.0 + x4 / 24.0 - x6 / 720.0
}

const fn sin(x: f32) -> f32 {
    let x3 = x * x * x;
    let x5 = x3 * x * x;
    let x7 = x5 * x * x;
    x - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0
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

#[test]
fn multiplying_by_a_translation_matrix() {
    const TRANSFORM: Matrix<4, 4> = translation(5.0, -3.0, 2.0);
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
