use crate::{
    matrices::Matrix,
    shapes::{HasTransform, Shape, TransformData},
    texture_maps::*,
    tuples::*,
};

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct CheckerPattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct RingPattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct GradientPattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct StripePattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct TestPattern {
    transform: TransformData,
}

// A UV pattern projected onto a shape through a single mapping (spherical,
// planar, or cylindrical).
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct TextureMapPattern {
    uv: UvPattern,
    mapping: UvMapping,
    transform: TransformData,
}

// Six UV patterns, one per cube face, selected per point by `face_from_point`.
// Faces are stored in the order left, front, right, back, up, down.
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct CubeMapPattern {
    faces: [UvPattern; 6],
    transform: TransformData,
}

impl TestPattern {
    fn color(&self, point: Point) -> Color {
        Color {
            r: point.x(),
            g: point.y(),
            b: point.z(),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Pattern {
    Test(TestPattern),
    Stripe(StripePattern),
    Gradient(GradientPattern),
    Ring(RingPattern),
    Checker(CheckerPattern),
    Texture(TextureMapPattern),
    Cube(CubeMapPattern),
}

impl HasTransform for Pattern {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        match self {
            Pattern::Test(test_pattern) => test_pattern.transform.set_transform(transform),
            Pattern::Stripe(stripe_pattern) => stripe_pattern.transform.set_transform(transform),
            Pattern::Gradient(gradient_pattern) => {
                gradient_pattern.transform.set_transform(transform)
            }
            Pattern::Ring(ring_pattern) => ring_pattern.transform.set_transform(transform),
            Pattern::Checker(checker_pattern) => checker_pattern.transform.set_transform(transform),
            Pattern::Texture(texture) => texture.transform.set_transform(transform),
            Pattern::Cube(cube) => cube.transform.set_transform(transform),
        }
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        match self {
            Pattern::Test(test_pattern) => test_pattern.transform.get_transform(),
            Pattern::Stripe(stripe_pattern) => stripe_pattern.transform.get_transform(),
            Pattern::Gradient(gradient_pattern) => gradient_pattern.transform.get_transform(),
            Pattern::Ring(ring_pattern) => ring_pattern.transform.get_transform(),
            Pattern::Checker(checker_pattern) => checker_pattern.transform.get_transform(),
            Pattern::Texture(texture) => texture.transform.get_transform(),
            Pattern::Cube(cube) => cube.transform.get_transform(),
        }
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        match self {
            Pattern::Test(test_pattern) => test_pattern.transform.get_inverse_transform(),
            Pattern::Stripe(stripe_pattern) => stripe_pattern.transform.get_inverse_transform(),
            Pattern::Gradient(gradient_pattern) => {
                gradient_pattern.transform.get_inverse_transform()
            }
            Pattern::Ring(ring_pattern) => ring_pattern.transform.get_inverse_transform(),
            Pattern::Checker(checker_pattern) => checker_pattern.transform.get_inverse_transform(),
            Pattern::Texture(texture) => texture.transform.get_inverse_transform(),
            Pattern::Cube(cube) => cube.transform.get_inverse_transform(),
        }
    }
}

impl CheckerPattern {
    fn new(a: Color, b: Color) -> Self {
        Self {
            a,
            b,
            transform: TransformData::new(Matrix::identity(), None),
        }
    }
    fn color(&self, point: Point) -> Color {
        // `stable_floor` absorbs the ~1e-15 error a plane's hit point carries
        // (see StableFloor); `rem_euclid` keeps the parity non-negative.
        let parity = point.x().stable_floor() + point.y().stable_floor() + point.z().stable_floor();
        if parity.rem_euclid(2.0) == 0.0 {
            self.a
        } else {
            self.b
        }
    }
}

impl RingPattern {
    fn new(a: Color, b: Color) -> Self {
        Self {
            a,
            b,
            transform: TransformData::new(Matrix::identity(), None),
        }
    }
    fn color(&self, point: Point) -> Color {
        if (point.x().powi(2) + point.z().powi(2)).sqrt().floor() % 2.0 == 0.0 {
            return self.a;
        }
        self.b
    }
}

impl GradientPattern {
    fn new(a: Color, b: Color) -> Self {
        Self {
            a,
            b,
            transform: TransformData::new(Matrix::identity(), None),
        }
    }
    fn color(&self, point: Point) -> Color {
        let distance = self.b - self.a;
        let fraction = point.x - point.x.floor();

        self.a + distance * fraction
    }
}

impl StripePattern {
    fn new(a: Color, b: Color) -> Self {
        Self {
            a,
            b,
            transform: TransformData::new(Matrix::identity(), None),
        }
    }
    fn color(&self, point: Point) -> Color {
        if point.x().floor() % 2.0 == 0.0 {
            return self.a;
        }
        self.b
    }
}

impl Pattern {
    pub fn test_pattern() -> Self {
        Pattern::Test(TestPattern {
            transform: TransformData::new(Matrix::identity(), None),
        })
    }
    pub fn stripe_pattern(a: Color, b: Color) -> Self {
        Pattern::Stripe(StripePattern::new(a, b))
    }
    pub fn gradient_pattern(a: Color, b: Color) -> Self {
        Pattern::Gradient(GradientPattern::new(a, b))
    }
    pub fn ring_pattern(a: Color, b: Color) -> Self {
        Pattern::Ring(RingPattern::new(a, b))
    }
    pub fn checker_pattern(a: Color, b: Color) -> Self {
        Pattern::Checker(CheckerPattern::new(a, b))
    }
    // A UV pattern projected through `mapping` (spherical/planar/cylindrical).
    pub fn texture_map(uv: UvPattern, mapping: UvMapping) -> Self {
        Pattern::Texture(TextureMapPattern {
            uv,
            mapping,
            transform: TransformData::new(Matrix::identity(), None),
        })
    }
    // Six UV patterns, one per cube face (left, front, right, back, up, down).
    pub fn cube_map(
        left: UvPattern,
        front: UvPattern,
        right: UvPattern,
        back: UvPattern,
        up: UvPattern,
        down: UvPattern,
    ) -> Self {
        Pattern::Cube(CubeMapPattern {
            faces: [left, front, right, back, up, down],
            transform: TransformData::new(Matrix::identity(), None),
        })
    }
    pub fn pattern_at_shape(&self, object: &Shape, world_point: Point) -> Color {
        let object_point = match object.get_inverse_transform() {
            None => world_point,
            Some(inverse_transform) => inverse_transform * world_point,
        };
        let pattern_point = match self.get_inverse_transform() {
            None => object_point,
            Some(inverse_transform) => inverse_transform * object_point,
        };
        self.pattern_at(pattern_point)
    }
    pub fn pattern_at(&self, point: Point) -> Color {
        match self {
            Pattern::Test(test_pattern) => test_pattern.color(point),
            Pattern::Stripe(stripe_pattern) => stripe_pattern.color(point),
            Pattern::Gradient(gradient_pattern) => gradient_pattern.color(point),
            Pattern::Ring(ring_pattern) => ring_pattern.color(point),
            Pattern::Checker(checker_pattern) => checker_pattern.color(point),
            Pattern::Texture(texture) => {
                let (u, v) = texture.mapping.map(point);
                texture.uv.uv_pattern_at(u, v)
            }
            Pattern::Cube(cube) => {
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
                cube.faces[index].uv_pattern_at(u, v)
            }
        }
    }
    fn a(&self) -> Color {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.a,
            Pattern::Gradient(gradient_pattern) => gradient_pattern.a,
            Pattern::Ring(ring_pattern) => ring_pattern.a,
            Pattern::Checker(checker_pattern) => checker_pattern.a,
            _ => panic!("No 'a' color for {:?}", self),
        }
    }
    fn b(&self) -> Color {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.b,
            Pattern::Gradient(gradient_pattern) => gradient_pattern.b,
            Pattern::Ring(ring_pattern) => ring_pattern.b,
            Pattern::Checker(checker_pattern) => checker_pattern.b,
            _ => panic!("No 'b' color for {:?}", self),
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
        assert_eq!(pattern.a(), white);
        assert_eq!(pattern.b(), black);
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
        let mut object = Shape::sphere();
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
        let object = Shape::sphere();
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
        let mut object = Shape::sphere();
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
        let mut shape = Shape::sphere();
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
        let shape = Shape::sphere();
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
        let mut shape = Shape::sphere();
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
            UvPattern::checkers(16.0, 8.0, black, white),
            UvMapping::Spherical,
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
        let pattern = Pattern::cube_map(
            UvPattern::align_check(yellow, cyan, red, blue, brown), // left
            UvPattern::align_check(cyan, red, yellow, brown, green), // front
            UvPattern::align_check(red, yellow, purple, green, white), // right
            UvPattern::align_check(green, purple, cyan, white, blue), // back
            UvPattern::align_check(brown, cyan, purple, red, yellow), // up
            UvPattern::align_check(purple, brown, green, blue, white), // down
        );
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
