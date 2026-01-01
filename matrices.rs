use std::ops::Mul;

use crate::tuples::*;

#[derive(Debug, Clone, Copy)]
pub struct Matrix<const ROWS: usize, const COLS: usize> {
    data: [[f32; COLS]; ROWS],
}

impl<const ROWS: usize, const COLS: usize> Matrix<ROWS, COLS> {
    pub const fn new(data: [[f32; COLS]; ROWS]) -> Self {
        Self { data }
    }
    pub const fn init(value: f32) -> Self {
        Self::new([[value; COLS]; ROWS])
    }
    pub const fn identity() -> Self {
        let mut result = Self::init(0.0);
        let mut row = 0;
        let mut col = 0;
        while row < ROWS && col < COLS {
            result.set(row, col, 1.0);
            row += 1;
            col += 1;
        }
        result
    }
    pub const fn get(&self, row: usize, col: usize) -> f32 {
        self.data[row][col]
    }
    pub const fn set(&mut self, row: usize, col: usize, value: f32) -> () {
        self.data[row][col] = value;
    }
    pub const fn then(&self, b: Matrix<ROWS, COLS>) -> Matrix<ROWS, COLS> {
        mul(&b, self)
    }
}

pub const fn mul<const ROWS: usize, const COLS: usize>(
    a: &Matrix<ROWS, COLS>,
    b: &Matrix<ROWS, COLS>,
) -> Matrix<ROWS, COLS> {
    let mut m = Matrix::init(0.0);
    let mut i = 0;
    while i < ROWS {
        let mut j = 0;
        while j < COLS {
            let mut sum = 0.0;
            let mut k = 0;
            while k < COLS {
                sum += a.get(i, k) * b.get(k, j);
                k += 1;
            }
            m.set(i, j, sum);
            j += 1;
        }
        i += 1;
    }
    m
}

pub fn transpose<const ROWS: usize, const COLS: usize>(
    a: &Matrix<ROWS, COLS>,
) -> Matrix<COLS, ROWS> {
    let mut result = Matrix::init(0.0);
    for row in 0..ROWS {
        for col in 0..COLS {
            result.set(col, row, a.get(row, col));
        }
    }
    result
}

pub fn submatrix<const N: usize>(
    a: &Matrix<N, N>,
    row: usize,
    col: usize,
) -> Matrix<{ N - 1 }, { N - 1 }> {
    let mut result: Matrix<{ N - 1 }, { N - 1 }> = Matrix::init(0.0);
    let mut i = 0;
    for y in 0..N {
        if y == row {
            continue;
        }
        let mut j = 0;
        for x in 0..N {
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

pub fn minor<const N: usize>(a: &Matrix<N, N>, row: usize, col: usize) -> f32
where
    [(); N - 1]:,
    Matrix<{ N - 1 }, { N - 1 }>: Determinant,
{
    submatrix(a, row, col).determinant()
}

pub fn cofactor<const N: usize>(a: &Matrix<N, N>, row: usize, col: usize) -> f32
where
    [(); N - 1]:,
    Matrix<{ N - 1 }, { N - 1 }>: Determinant,
{
    if (row + col) % 2 == 0 {
        minor(a, row, col)
    } else {
        -minor(a, row, col)
    }
}

fn determinant_of_n<const N: usize>(a: &Matrix<N, N>) -> f32
where
    [(); N - 1]:,
    Matrix<{ N - 1 }, { N - 1 }>: Determinant,
{
    let mut result = 0.0;
    for n in 0..N {
        result += cofactor(a, 0, n) * a.get(0, n)
    }
    result
}

pub fn determinant<const N: usize>(a: &Matrix<N, N>) -> f32
where
    [(); N - 1]:,
    Matrix<{ N - 1 }, { N - 1 }>: Determinant,
    Matrix<{ N }, { N }>: Determinant,
{
    a.determinant()
}

pub fn is_invertible<const N: usize>(a: &Matrix<N, N>) -> bool
where
    [(); N - 1]:,
    Matrix<{ N - 1 }, { N - 1 }>: Determinant,
    Matrix<{ N }, { N }>: Determinant,
{
    determinant(a) != 0.0
}

pub fn inverse<const N: usize>(m: &Matrix<N, N>) -> Option<Matrix<N, N>>
where
    [(); N - 1]:,
    Matrix<{ N - 1 }, { N - 1 }>: Determinant,
    Matrix<{ N }, { N }>: Determinant,
{
    if !is_invertible(m) {
        return None;
    }

    let mut m2: Matrix<N, N> = Matrix::init(0.0);
    for row in 0..N {
        for col in 0..N {
            let c = cofactor(m, row, col);
            m2.set(col, row, c / determinant(m));
        }
    }
    Some(m2)
}

pub trait Determinant {
    fn determinant(&self) -> f32;
}

impl Determinant for Matrix<1, 1> {
    fn determinant(&self) -> f32 {
        self.get(0, 0)
    }
}

impl Determinant for Matrix<2, 2> {
    fn determinant(&self) -> f32 {
        self.get(0, 0) * self.get(1, 1) - self.get(0, 1) * self.get(1, 0)
    }
}
impl Determinant for Matrix<3, 3> {
    fn determinant(&self) -> f32 {
        determinant_of_n(self)
    }
}
impl Determinant for Matrix<4, 4> {
    fn determinant(&self) -> f32 {
        determinant_of_n(self)
    }
}

impl<const ROWS: usize, const COLS: usize> PartialEq for Matrix<ROWS, COLS> {
    fn eq(&self, other: &Self) -> bool {
        for row in 0..ROWS {
            for col in 0..COLS {
                if (self.get(row, col) - other.get(row, col)).abs() > EPSILON {
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
        mul(&self, &other)
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
#[test]
fn calculating_a_minor_of_a_3x3_matrix() {
    let a: Matrix<3, 3> = Matrix::new([[3.0, 5.0, 0.0], [2.0, -1.0, -7.0], [6.0, -1.0, 5.0]]);
    let b = submatrix(&a, 1, 0);
    assert_eq!(determinant(&b), 25.0);
    assert_eq!(minor(&a, 1, 0), 25.0);
}
#[test]
fn calculating_a_cofactor_of_a_3x3_matrix() {
    let a: Matrix<3, 3> = Matrix::new([[3.0, 5.0, 0.0], [2.0, -1.0, -7.0], [6.0, -1.0, 5.0]]);
    assert_eq!(minor(&a, 0, 0), -12.0);
    assert_eq!(cofactor(&a, 0, 0), -12.0);
    assert_eq!(minor(&a, 1, 0), 25.0);
    assert_eq!(cofactor(&a, 1, 0), -25.0);
}
#[test]
fn calculating_the_determinant_of_a_3x3_matrix() {
    let a: Matrix<3, 3> = Matrix::new([[1.0, 2.0, 6.0], [-5.0, 8.0, -4.0], [2.0, 6.0, 4.0]]);
    assert_eq!(cofactor(&a, 0, 0), 56.0);
    assert_eq!(cofactor(&a, 0, 1), 12.0);
    assert_eq!(cofactor(&a, 0, 2), -46.0);
    assert_eq!(determinant(&a), -196.0);
}
#[test]
fn calculating_the_determinant_of_a_4x4_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [-2.0, -8.0, 3.0, 5.0],
        [-3.0, 1.0, 7.0, 3.0],
        [1.0, 2.0, -9.0, 6.0],
        [-6.0, 7.0, 7.0, -9.0],
    ]);
    assert_eq!(cofactor(&a, 0, 0), 690.0);
    assert_eq!(cofactor(&a, 0, 1), 447.0);
    assert_eq!(cofactor(&a, 0, 2), 210.0);
    assert_eq!(cofactor(&a, 0, 3), 51.0);
    assert_eq!(determinant(&a), -4071.0);
}
#[test]
fn testing_an_invertible_matrix_for_invertability() {
    let a: Matrix<4, 4> = Matrix::new([
        [6.0, 4.0, 4.0, 4.0],
        [5.0, 5.0, 7.0, 6.0],
        [4.0, -9.0, 3.0, -7.0],
        [9.0, 1.0, 7.0, -6.0],
    ]);
    assert_eq!(determinant(&a), -2120.0);
    assert_eq!(is_invertible(&a), true);
}
#[test]
fn testing_a_noninvertible_matrix_for_invertibility() {
    let a: Matrix<4, 4> = Matrix::new([
        [-4.0, 2.0, -2.0, -3.0],
        [9.0, 6.0, 2.0, 6.0],
        [0.0, -5.0, 1.0, -5.0],
        [0.0, 0.0, 0.0, 0.0],
    ]);
    assert_eq!(determinant(&a), 0.0);
    assert_eq!(is_invertible(&a), false);
    assert_eq!(inverse(&a), None);
}
#[test]
fn calculating_the_inverse_of_a_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [-5.0, 2.0, 6.0, -8.0],
        [1.0, -5.0, 1.0, 8.0],
        [7.0, 7.0, -6.0, -7.0],
        [1.0, -3.0, 7.0, 4.0],
    ]);
    assert_ne!(inverse(&a), None);
    let b: Matrix<4, 4> = inverse(&a).unwrap();
    assert_eq!(determinant(&a), 532.0);
    assert_eq!(cofactor(&a, 2, 3), -160.0);
    assert_eq!(b.get(3, 2), -160.0 / 532.0);
    assert_eq!(cofactor(&a, 3, 2), 105.0);
    assert_eq!(b.get(2, 3), 105.0 / 532.0);
    assert_eq!(
        b,
        Matrix::new([
            [0.21805, 0.45113, 0.24060, -0.04511],
            [-0.80827, -1.45677, -0.44361, 0.52068],
            [-0.07895, -0.22368, -0.05253, 0.19737],
            [-0.52256, -0.81391, -0.30075, 0.30639]
        ])
    )
}
#[test]
fn calculating_the_inverse_of_another_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [8.0, -5.0, 9.0, 2.0],
        [7.0, 5.0, 6.0, 1.0],
        [-6.0, 0.0, 9.0, 6.0],
        [-3.0, 0.0, -9.0, -4.0],
    ]);
    assert_eq!(
        inverse(&a),
        Some(Matrix::new([
            [-0.15385, -0.15385, -0.28205, -0.53846],
            [-0.07692, 0.12308, 0.02564, 0.03077],
            [0.35897, 0.35897, 0.43590, 0.92308],
            [-0.69231, -0.69231, -0.76923, -1.92308]
        ]))
    )
}
#[test]
fn calculating_the_inverse_of_third_matrix() {
    let a: Matrix<4, 4> = Matrix::new([
        [9.0, 3.0, 0.0, 9.0],
        [-5.0, -2.0, -6.0, -3.0],
        [-4.0, 9.0, 6.0, 4.0],
        [-7.0, 6.0, 6.0, 2.0],
    ]);
    assert_eq!(
        inverse(&a),
        Some(Matrix::new([
            [-0.04074, -0.07778, 0.14444, -0.22222],
            [-0.07778, 0.03333, 0.36667, -0.33333],
            [-0.02901, -0.14630, -0.10926, 0.12963],
            [0.17778, 0.06667, -0.26667, 0.33333]
        ]))
    )
}
#[test]
fn multiplying_a_product_by_its_inverse() {
    let a: Matrix<4, 4> = Matrix::new([
        [3.0, -9.0, 7.0, 3.0],
        [3.0, -8.0, 2.0, -9.0],
        [-4.0, 4.0, 4.0, 1.0],
        [-6.0, 5.0, -1.0, 1.0],
    ]);
    let b: Matrix<4, 4> = Matrix::new([
        [8.0, 2.0, 2.0, 2.0],
        [3.0, -1.0, 7.0, 0.0],
        [7.0, 0.0, 5.0, 4.0],
        [6.0, -2.0, 0.0, 5.0],
    ]);
    let c: Matrix<4, 4> = a * b;
    assert_ne!(inverse(&b), None);
    assert_eq!(c * inverse(&b).unwrap(), a);
}
