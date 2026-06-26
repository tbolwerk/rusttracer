use crate::{
    matrices::{inverse, Matrix},
    shapes::{HasTransform, Primitive},
    texture_maps::*,
    tuples::*,
};

const fn black() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    }
}

// Flat tagged struct for rust-gpu/SPIR-V compatibility. `kind` selects which
// fields are meaningful:
//   0 = NONE (no pattern)
//   1 = stripe   (a, b)
//   2 = gradient (a, b)
//   3 = ring     (a, b)
//   4 = checker  (a, b)
//   5 = texture  (uv + mapping)
//   6 = cube     (faces[6])
//   7 = test
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pattern {
    pub kind: u32,
    pub a: Color,
    pub b: Color,
    pub transform: Matrix<4, 4>,
    pub inverse: Matrix<4, 4>,
    pub uv: UvFace,
    pub mapping: u32,
    pub faces: [UvFace; 6],
}

impl HasTransform for Pattern {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse = inverse(&transform).unwrap_or(Matrix::identity());
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        self.transform
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        Some(self.inverse)
    }
}

impl Pattern {
    pub const fn none() -> Self {
        let face = UvFace::checkers(0.0, 0.0, black(), black());
        Pattern {
            kind: 0,
            a: black(),
            b: black(),
            transform: Matrix::identity(),
            inverse: Matrix::identity(),
            uv: face,
            mapping: MAPPING_SPHERICAL,
            faces: [face; 6],
        }
    }
    fn base() -> Self {
        Pattern::none()
    }
    pub fn test_pattern() -> Self {
        Pattern {
            kind: 7,
            ..Pattern::base()
        }
    }
    pub fn stripe_pattern(a: Color, b: Color) -> Self {
        Pattern {
            kind: 1,
            a,
            b,
            ..Pattern::base()
        }
    }
    pub fn gradient_pattern(a: Color, b: Color) -> Self {
        Pattern {
            kind: 2,
            a,
            b,
            ..Pattern::base()
        }
    }
    pub fn ring_pattern(a: Color, b: Color) -> Self {
        Pattern {
            kind: 3,
            a,
            b,
            ..Pattern::base()
        }
    }
    pub fn checker_pattern(a: Color, b: Color) -> Self {
        Pattern {
            kind: 4,
            a,
            b,
            ..Pattern::base()
        }
    }
    // A UV pattern projected through `mapping` (spherical/planar/cylindrical).
    pub fn texture_map(uv: UvFace, mapping: u32) -> Self {
        Pattern {
            kind: 5,
            uv,
            mapping,
            ..Pattern::base()
        }
    }
    // Six UV patterns, one per cube face (left, front, right, back, up, down).
    pub fn cube_map(faces: [UvFace; 6]) -> Self {
        Pattern {
            kind: 6,
            faces,
            ..Pattern::base()
        }
    }
    pub fn pattern_at_shape(&self, object: &Primitive, world_point: Point) -> Color {
        let object_point = match object.get_inverse_transform() {
            None => world_point,
            Some(inverse_transform) => inverse_transform * world_point,
        };
        let pattern_point = self.inverse * object_point;
        self.pattern_at(pattern_point)
    }
    pub fn pattern_at(&self, point: Point) -> Color {
        match self.kind {
            7 => Color {
                r: point.x(),
                g: point.y(),
                b: point.z(),
            },
            1 => {
                // stripe
                if point.x().floor() % 2.0 == 0.0 {
                    self.a
                } else {
                    self.b
                }
            }
            2 => {
                // gradient
                let distance = self.b - self.a;
                let fraction = point.x - point.x.floor();
                self.a + distance * fraction
            }
            3 => {
                // ring
                if (point.x().powi(2) + point.z().powi(2)).sqrt().floor() % 2.0 == 0.0 {
                    self.a
                } else {
                    self.b
                }
            }
            4 => {
                // checker
                // `stable_floor` absorbs the ~1e-15 error a plane's hit point
                // carries (see StableFloor); `rem_euclid` keeps parity non-negative.
                let parity = point.x().stable_floor()
                    + point.y().stable_floor()
                    + point.z().stable_floor();
                if parity.rem_euclid(2.0) == 0.0 {
                    self.a
                } else {
                    self.b
                }
            }
            5 => {
                // texture map
                let (u, v) = uv_map(point, self.mapping);
                self.uv.uv_pattern_at(u, v)
            }
            6 => {
                // cube map
                let face = face_from_point(point);
                let index = match face {
                    CubeFace::Left => 0,
                    CubeFace::Front => 1,
                    CubeFace::Right => 2,
                    CubeFace::Back => 3,
                    CubeFace::Up => 4,
                    CubeFace::Down => 5,
                };
                let (u, v) = cube_uv(face, point);
                self.faces[index].uv_pattern_at(u, v)
            }
            _ => self.a,
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::transformations::{scaling, translation};

    use super::*;
    fn background() -> (Color, Color) {
        let black = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        let white = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        };
        (black, white)
    }
    #[test]
    fn creating_a_striped_pattern() {
        let (black, white) = background();
        let pattern = Pattern::stripe_pattern(white, black);
        assert_eq!(pattern.a, white);
        assert_eq!(pattern.b, black);
    }
    #[test]
    fn a_stripe_pattern_is_constant_in_y() {
        let (black, white) = background();
        let pattern = Pattern::stripe_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 2.0,
                z: 0.0,
            }),
            white
        );
    }
    #[test]
    fn a_stripe_pattern_is_constant_in_z() {
        let (black, white) = background();
        let pattern = Pattern::stripe_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            }),
            white
        );
    }
    #[test]
    fn a_stripe_pattern_is_alternates_in_x() {
        let (black, white) = background();
        let pattern = Pattern::stripe_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.9,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: -0.1,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: -1.1,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
    }
    #[test]
    fn stripes_with_an_object_transformation() {
        let (black, white) = background();
        let mut object = Primitive::sphere();
        object.set_transform(scaling(2.0, 2.0, 2.0));
        let pattern = Pattern::stripe_pattern(white, black);
        let c = pattern.pattern_at_shape(
            &object,
            Point {
                x: 1.5,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_eq!(c, white)
    }
    #[test]
    fn stripes_with_a_pattern_transformation() {
        let (black, white) = background();
        let object = Primitive::sphere();
        let mut pattern = Pattern::stripe_pattern(white, black);
        pattern.set_transform(scaling(2.0, 2.0, 2.0));
        let c = pattern.pattern_at_shape(
            &object,
            Point {
                x: 1.5,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_eq!(c, white)
    }
    #[test]
    fn stripes_with_both_an_object_and_a_pattern_transformation() {
        let (black, white) = background();
        let mut object = Primitive::sphere();
        object.set_transform(scaling(2.0, 2.0, 2.0));
        let mut pattern = Pattern::stripe_pattern(white, black);
        pattern.set_transform(translation(0.5, 0.0, 0.0));
        let c = pattern.pattern_at_shape(
            &object,
            Point {
                x: 2.5,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_eq!(c, white)
    }
    #[test]
    fn the_default_pattern_transformation() {
        let pattern = Pattern::test_pattern();
        assert_eq!(pattern.get_transform(), Matrix::identity());
    }
    #[test]
    fn assigning_a_transformation() {
        let mut pattern = Pattern::test_pattern();
        pattern.set_transform(translation(1.0, 2.0, 3.0));
        assert_eq!(pattern.get_transform(), translation(1.0, 2.0, 3.0));
    }
    #[test]
    fn a_pattern_with_an_object_transformation() {
        let mut shape = Primitive::sphere();
        shape.set_transform(scaling(2.0, 2.0, 2.0));
        let pattern = Pattern::test_pattern();
        let c = pattern.pattern_at_shape(
            &shape,
            Point {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            },
        );
        assert_eq!(
            c,
            Color {
                r: 1.0,
                g: 1.5,
                b: 2.0
            }
        );
    }
    #[test]
    fn a_pattern_with_an_pattern_transformation() {
        let shape = Primitive::sphere();
        let mut pattern = Pattern::test_pattern();
        pattern.set_transform(scaling(2.0, 2.0, 2.0));
        let c = pattern.pattern_at_shape(
            &shape,
            Point {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            },
        );
        assert_eq!(
            c,
            Color {
                r: 1.0,
                g: 1.5,
                b: 2.0
            }
        );
    }
    #[test]
    fn a_pattern_with_both_an_object_and_a_pattern_transformation() {
        let mut shape = Primitive::sphere();
        shape.set_transform(scaling(2.0, 2.0, 2.0));
        let mut pattern = Pattern::test_pattern();
        pattern.set_transform(translation(0.5, 1.0, 1.5));
        let c = pattern.pattern_at_shape(
            &shape,
            Point {
                x: 2.5,
                y: 3.0,
                z: 3.5,
            },
        );
        assert_eq!(
            c,
            Color {
                r: 0.75,
                g: 0.5,
                b: 0.25
            }
        );
    }
    #[test]
    fn a_gradient_linearly_interpolates_between_colors() {
        let (black, white) = background();
        let pattern = Pattern::gradient_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.25,
                y: 0.0,
                z: 0.0,
            }),
            Color {
                r: 0.75,
                g: 0.75,
                b: 0.75,
            }
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.5,
                y: 0.0,
                z: 0.0,
            }),
            Color {
                r: 0.5,
                g: 0.5,
                b: 0.5,
            }
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.75,
                y: 0.0,
                z: 0.0,
            }),
            Color {
                r: 0.25,
                g: 0.25,
                b: 0.25,
            }
        );
    }
    #[test]
    fn a_ring_should_extend_in_both_x_and_y() {
        let (black, white) = background();
        let pattern = Pattern::ring_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }),
            black
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.708,
                y: 0.0,
                z: 0.708,
            }),
            black
        );
    }
    #[test]
    fn checker_should_repeat_in_x() {
        let (black, white) = background();
        let pattern = Pattern::checker_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.99,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 1.01,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
    }
    #[test]
    fn checker_should_repeat_in_y() {
        let (black, white) = background();
        let pattern = Pattern::checker_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.99,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 1.01,
                z: 0.0,
            }),
            black
        );
    }
    #[test]
    fn checker_should_repeat_in_z() {
        let (black, white) = background();
        let pattern = Pattern::checker_pattern(white, black);
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.99,
            }),
            white
        );
        assert_eq!(
            pattern.pattern_at(Point {
                x: 0.0,
                y: 0.0,
                z: 1.01,
            }),
            black
        );
    }

    #[test]
    fn checkers_applied_through_a_spherical_texture_map() {
        let (black, white) = background();
        let pattern = Pattern::texture_map(
            UvFace::checkers(16.0, 8.0, black, white),
            MAPPING_SPHERICAL,
        );
        // (point on the unit sphere -> expected color), from the book.
        let cases = [
            (Point { x: 0.4315, y: 0.4670, z: 0.7719 }, white),
            (Point { x: -0.9654, y: 0.2552, z: -0.0534 }, black),
            (Point { x: 0.1039, y: 0.7090, z: 0.6975 }, white),
            (Point { x: -0.4986, y: -0.7856, z: -0.3663 }, black),
            (Point { x: -0.0317, y: -0.9395, z: 0.3411 }, black),
            (Point { x: 0.4809, y: -0.7721, z: 0.4154 }, black),
            (Point { x: 0.0285, y: -0.9612, z: -0.2745 }, black),
            (Point { x: -0.5734, y: -0.2162, z: -0.7903 }, white),
            (Point { x: 0.7688, y: -0.1470, z: 0.6223 }, black),
            (Point { x: -0.7652, y: 0.2175, z: 0.6060 }, black),
        ];
        for (p, expected) in cases {
            assert_eq!(pattern.pattern_at(p), expected, "p={p:?}");
        }
    }

    #[test]
    fn finding_the_colors_on_a_mapped_cube() {
        let red = Color { r: 1.0, g: 0.0, b: 0.0 };
        let yellow = Color { r: 1.0, g: 1.0, b: 0.0 };
        let brown = Color { r: 1.0, g: 0.5, b: 0.0 };
        let green = Color { r: 0.0, g: 1.0, b: 0.0 };
        let cyan = Color { r: 0.0, g: 1.0, b: 1.0 };
        let blue = Color { r: 0.0, g: 0.0, b: 1.0 };
        let purple = Color { r: 1.0, g: 0.0, b: 1.0 };
        let white = Color { r: 1.0, g: 1.0, b: 1.0 };
        let pattern = Pattern::cube_map([
            UvFace::align_check(yellow, cyan, red, blue, brown), // left
            UvFace::align_check(cyan, red, yellow, brown, green), // front
            UvFace::align_check(red, yellow, purple, green, white), // right
            UvFace::align_check(green, purple, cyan, white, blue), // back
            UvFace::align_check(brown, cyan, purple, red, yellow), // up
            UvFace::align_check(purple, brown, green, blue, white), // down
        ]);
        let cases = [
            // Left
            (Point { x: -1.0, y: 0.0, z: 0.0 }, yellow),
            (Point { x: -1.0, y: 0.9, z: -0.9 }, cyan),
            (Point { x: -1.0, y: 0.9, z: 0.9 }, red),
            (Point { x: -1.0, y: -0.9, z: -0.9 }, blue),
            (Point { x: -1.0, y: -0.9, z: 0.9 }, brown),
            // Front
            (Point { x: 0.0, y: 0.0, z: 1.0 }, cyan),
            (Point { x: -0.9, y: 0.9, z: 1.0 }, red),
            (Point { x: 0.9, y: 0.9, z: 1.0 }, yellow),
            (Point { x: -0.9, y: -0.9, z: 1.0 }, brown),
            (Point { x: 0.9, y: -0.9, z: 1.0 }, green),
            // Right
            (Point { x: 1.0, y: 0.0, z: 0.0 }, red),
            (Point { x: 1.0, y: 0.9, z: 0.9 }, yellow),
            (Point { x: 1.0, y: 0.9, z: -0.9 }, purple),
            (Point { x: 1.0, y: -0.9, z: 0.9 }, green),
            (Point { x: 1.0, y: -0.9, z: -0.9 }, white),
            // Back
            (Point { x: 0.0, y: 0.0, z: -1.0 }, green),
            (Point { x: 0.9, y: 0.9, z: -1.0 }, purple),
            (Point { x: -0.9, y: 0.9, z: -1.0 }, cyan),
            (Point { x: 0.9, y: -0.9, z: -1.0 }, white),
            (Point { x: -0.9, y: -0.9, z: -1.0 }, blue),
            // Up
            (Point { x: 0.0, y: 1.0, z: 0.0 }, brown),
            (Point { x: -0.9, y: 1.0, z: -0.9 }, cyan),
            (Point { x: 0.9, y: 1.0, z: -0.9 }, purple),
            (Point { x: -0.9, y: 1.0, z: 0.9 }, red),
            (Point { x: 0.9, y: 1.0, z: 0.9 }, yellow),
            // Down
            (Point { x: 0.0, y: -1.0, z: 0.0 }, purple),
            (Point { x: -0.9, y: -1.0, z: 0.9 }, brown),
            (Point { x: 0.9, y: -1.0, z: 0.9 }, green),
            (Point { x: -0.9, y: -1.0, z: -0.9 }, blue),
            (Point { x: 0.9, y: -1.0, z: -0.9 }, white),
        ];
        for (p, expected) in cases {
            assert_eq!(pattern.pattern_at(p), expected, "p={p:?}");
        }
    }
}
