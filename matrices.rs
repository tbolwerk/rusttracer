use std::ops::Mul;

use crate::tuples::*;

#[derive(Debug, Clone, Copy)]
struct Matrix<const ROWS: usize, const COLS: usize> {
    data: [[f32; COLS]; ROWS],
}

impl<const ROWS: usize, const COLS: usize> Matrix<ROWS, COLS> {
    fn new(data: [[f32; COLS]; ROWS]) -> Self {
        Self { data }
    }
    fn init(value: f32) -> Self {
        Self::new([[value; COLS]; ROWS])
    }
    fn identity() -> Self {
        let mut result = Self::init(0.0);
        for row in 0..ROWS {
            result.set(row, row, 1.0);
        }
        result
    }
    fn get(&self, row: usize, col: usize) -> f32 {
        self.data[row][col]
    }
    fn set(&mut self, row: usize, col: usize, value: f32) -> () {
        self.data[row][col] = value;
    }
}

fn transpose<const ROWS: usize, const COLS: usize>(a: &Matrix<ROWS, COLS>) -> Matrix<COLS, ROWS> {
    let mut result = Matrix::init(0.0);
    for row in 0..ROWS {
        for col in 0..COLS {
            result.set(col, row, a.get(row, col));
        }
    }
    result
}

fn determinant(a: &Matrix<2, 2>) -> f32 {
    a.get(0, 0) * a.get(1, 1) - a.get(0, 1) * a.get(1, 0)
}

fn submatrix<const ROWS: usize, const COLS: usize>(
    a: &Matrix<ROWS, COLS>,
    row: usize,
    col: usize,
) -> Matrix<{ ROWS - 1 }, { COLS - 1 }> {
    let mut result: Matrix<{ ROWS - 1 }, { COLS - 1 }> = Matrix::init(0.0);
    let mut i = 0;
    let mut j = 0;
    for y in 0..ROWS {
        if y == row {
            continue;
        }
        j = 0;
        for x in 0..COLS {
            if x == col {
                continue;
            }
            result.set(i, j, a.get(y, x));
            j += 1;
        }
        i += 1;
    }
    result
}

impl<const ROWS: usize, const COLS: usize> PartialEq for Matrix<ROWS, COLS> {
    fn eq(&self, other: &Self) -> bool {
        for row in 0..ROWS {
            for col in 0..COLS {
                if self.data[row][col] != other.data[row][col] {
                    return false;
                }
            }
        }
        return true;
    }
}

impl<const ROWS: usize, const COLS: usize> Mul for Matrix<ROWS, COLS> {
    type Output = Matrix<ROWS, COLS>;
    fn mul(self, other: Self) -> Self::Output {
        let mut result: Matrix<ROWS, COLS> = Matrix::init(0.0);
        for row in 0..ROWS {
            for col in 0..COLS {
                for k in 0..COLS {
                    let a = self.data[row][k];
                    let b = other.data[k][col];
                    result.data[row][col] += a * b;
                }
            }
        }
        result
    }
}

impl<const ROWS: usize, const COLS: usize> Mul<Tuple> for Matrix<ROWS, COLS> {
    type Output = Tuple;
    fn mul(self, other: Tuple) -> Self::Output {
        let mut result = Tuple::init(0.0);
        for row in 0..ROWS {
            let mut cel = 0.0;
            for col in 0..COLS {
                let a = self.data[row][col];
                let b = other.get(col);
                cel += a * b;
                result.set(row, cel);
            }
        }
        result
    }
}

#[test]
fn constructing_and_inspecting_a_4x4_matrix() {
    let m: Matrix<4, 4> = Matrix::new([
        [1.0, 2.0, 3.0, 4.0],
        [5.5, 6.5, 7.5, 8.5],
        [9.0, 10.0, 11.0, 12.0],
        [13.5, 14.5, 15.5, 16.5],
    ]);
    assert_eq!(m.get(0, 0), 1.0);
    assert_eq!(m.get(0, 3), 4.0);
    assert_eq!(m.get(1, 0), 5.5);
    assert_eq!(m.get(1, 2), 7.5);
    assert_eq!(m.get(2, 2), 11.0);
    assert_eq!(m.get(3, 0), 13.5);
    assert_eq!(m.get(3, 2), 15.5);
}
#[test]
fn a_2x2_matrix_ought_to_be_representable() {
    let m: Matrix<2, 2> = Matrix::new([[-3.0, 5.0], [1.0, -2.0]]);
    assert_eq!(m.get(0, 0), -3.0);
    assert_eq!(m.get(0, 1), 5.0);
    assert_eq!(m.get(1, 0), 1.0);
    assert_eq!(m.get(1, 1), -2.0);
}
#[test]
fn a_3x3_matrix_ought_to_be_representable() {
    let m: Matrix<3, 3> = Matrix::new([[-3.0, 5.0, 0.0], [1.0, -2.0, -7.0], [1.0, 1.0, 1.0]]);
    assert_eq!(m.get(0, 0), -3.0);
    assert_eq!(m.get(1, 1), -2.0);
    assert_eq!(m.get(2, 2), 1.0);
}
#[test]
fn matrix_equality_with_identical_matrices() {
    let a: Matrix<4, 4> = Matrix::new([
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 8.0, 7.0, 6.0],
        [5.0, 4.0, 3.0, 2.0],
    ]);
    let b: Matrix<4, 4> = Matrix::new([
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 8.0, 7.0, 6.0],
        [5.0, 4.0, 3.0, 2.0],
    ]);
    assert_eq!(a, b);
}
#[test]
fn matrix_equality_with_different_matrices() {
    let a: Matrix<4, 4> = Matrix::new([
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 8.0, 7.0, 6.0],
        [5.0, 4.0, 3.0, 2.0],
    ]);
    let b: Matrix<4, 4> = Matrix::new([
        [2.0, 3.0, 4.0, 5.0],
        [6.0, 7.0, 8.0, 9.0],
        [8.0, 7.0, 6.0, 5.0],
        [4.0, 3.0, 2.0, 1.0],
    ]);
    assert_ne!(a, b);
}
#[test]
fn multiplying_two_matrices() {
    let a: Matrix<4, 4> = Matrix::new([
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 8.0, 7.0, 6.0],
        [5.0, 4.0, 3.0, 2.0],
    ]);
    let b: Matrix<4, 4> = Matrix::new([
        [-2.0, 1.0, 2.0, 3.0],
        [3.0, 2.0, 1.0, -1.0],
        [4.0, 3.0, 6.0, 5.0],
        [1.0, 2.0, 7.0, 8.0],
    ]);
    assert_eq!(
        a * b,
        Matrix::new([
            [20.0, 22.0, 50.0, 48.0],
            [44.0, 54.0, 114.0, 108.0],
            [40.0, 58.0, 110.0, 102.0],
            [16.0, 26.0, 46.0, 42.0]
        ])
    );
}
#[test]
fn a_matrix_multiplied_by_a_tuple() {
    let a: Matrix<4, 4> = Matrix::new([
        [1.0, 2.0, 3.0, 4.0],
        [2.0, 4.0, 4.0, 2.0],
        [8.0, 6.0, 4.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let b = Tuple::new(1.0, 2.0, 3.0, 1.0);
    assert_eq!(a * b, Tuple::new(18.0, 24.0, 33.0, 1.0));
}
#[test]
fn multiplying_a_matrix_by_the_identity_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [0.0, 1.0, 2.0, 4.0],
        [1.0, 2.0, 4.0, 8.0],
        [2.0, 4.0, 8.0, 16.0],
        [4.0, 8.0, 16.0, 32.0],
    ]);
    let identity_matrix: Matrix<4, 4> = Matrix::identity();
    assert_eq!(a * identity_matrix, a);
}
#[test]
fn transpose_a_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [0.0, 9.0, 3.0, 0.0],
        [9.0, 8.0, 0.0, 8.0],
        [1.0, 8.0, 5.0, 3.0],
        [0.0, 0.0, 5.0, 8.0],
    ]);
    assert_eq!(
        transpose(&a),
        Matrix::new([
            [0.0, 9.0, 1.0, 0.0],
            [9.0, 8.0, 8.0, 0.0],
            [3.0, 0.0, 5.0, 5.0],
            [0.0, 8.0, 3.0, 8.0]
        ])
    );
}
#[test]
fn transpose_the_identity_matrix() {
    let a: Matrix<4, 4> = transpose(&Matrix::identity());
    assert_eq!(a, Matrix::identity());
}
#[test]
fn calculating_the_determinant_of_a_2x2_matrix() {
    let a: Matrix<2, 2> = Matrix::new([[1.0, 5.0], [-3.0, 2.0]]);
    assert_eq!(determinant(&a), 17.0);
}
#[test]
fn a_submatrix_of_a_3x3_matrix_is_a_2x2_matrix() {
    let a: Matrix<3, 3> = Matrix::new([[1.0, 5.0, 0.0], [-3.0, 2.0, 7.0], [0.0, 6.0, -3.0]]);
    assert_eq!(submatrix(&a, 0, 2), Matrix::new([[-3.0, 2.0], [0.0, 6.0],]));
}
#[test]
fn a_submatrix_of_a_4x4_matrix_is_a_3x3_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [-6.0, 1.0, 1.0, 6.0],
        [-8.0, 5.0, 8.0, 6.0],
        [-1.0, 0.0, 8.0, 2.0],
        [-7.0, 1.0, -1.0, 1.0],
    ]);
    assert_eq!(
        submatrix(&a, 2, 1),
        Matrix::new([[-6.0, 1.0, 6.0], [-8.0, 8.0, 6.0], [-7.0, -1.0, 1.0]])
    );
}
