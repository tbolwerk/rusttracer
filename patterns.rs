use crate::{
    matrices::{inverse, Matrix},
    shapes::{HasTransform, Shape, TransformData},
    tuples::*,
};

#[derive(PartialEq, Debug, Clone)]
struct CheckerPattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
struct RingPattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
struct GradientPattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
struct StripePattern {
    a: Color,
    b: Color,
    transform: TransformData,
}

#[derive(PartialEq, Debug, Clone)]
struct TestPattern {
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
        }
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        match self {
            Pattern::Test(test_pattern) => test_pattern.transform.get_transform(),
            Pattern::Stripe(stripe_pattern) => stripe_pattern.transform.get_transform(),
            Pattern::Gradient(gradient_pattern) => gradient_pattern.transform.get_transform(),
            Pattern::Ring(ring_pattern) => ring_pattern.transform.get_transform(),
            Pattern::Checker(checker_pattern) => checker_pattern.transform.get_transform(),
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
        if (point.x().floor() + point.y().floor() + point.z().floor()) % 2.0 == 0.0 {
            return self.a;
        }
        self.b
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
    fn test_pattern() -> Self {
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
}
