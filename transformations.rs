use crate::matrices::{inverse, Matrix};
use crate::tuples::Tuple;

fn translation(x: f32, y: f32, z: f32) -> Matrix<4, 4> {
    let mut m = Matrix::identity();
    m.set(0, 3, x);
    m.set(1, 3, y);
    m.set(2, 3, z);
    m
}

#[test]
fn multiplying_by_a_translation_matrix() {
    let transform = translation(5.0, -3.0, 2.0);
    println!("{:#?}", transform);
    let p = Tuple::point(-3.0, 4.0, 5.0);
    assert_eq!(transform * p, Tuple::point(2.0, 1.0, 7.0));
}
#[test]
fn multiplying_by_the_inverse_of_a_translation_matrix() {
    let transform = translation(5.0, -3.0, 2.0);
    let inv = inverse(&transform).unwrap();
    let p = Tuple::point(-3.0, 4.0, 5.0);
    assert_eq!(inv * p, Tuple::point(-8.0, 7.0, 3.0));
}
#[test]
fn translation_does_not_affect_vectors() {
    let transform = translation(5.0, -3.0, 2.0);
    let v = Tuple::vector(-3.0, 4.0, 5.0);
    assert_eq!(transform * v, v);
}
