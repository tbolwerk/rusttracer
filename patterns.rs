use crate::{
    matrices::{inverse, Matrix},
    shapes::{HasTransform, Shape},
    tuples::*,
};

#[derive(PartialEq, Debug, Clone)]
struct StripePattern {
    a: Color,
    b: Color,
    transform: Matrix<4, 4>,
    inverse_transform: Option<Matrix<4, 4>>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Pattern {
    Stripe(StripePattern),
}

impl HasTransform for StripePattern {
    fn set_transform(&mut self, transform: crate::matrices::Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse_transform = inverse(&transform);
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        self.transform
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        self.inverse_transform
    }
}

impl HasTransform for Pattern {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.set_transform(transform),
        }
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.get_transform(),
        }
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.get_inverse_transform(),
        }
    }
}

impl StripePattern {
    fn new(a: Color, b: Color) -> Self {
        Self {
            a,
            b,
            transform: Matrix::identity(),
            inverse_transform: None,
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
    pub fn stripe_pattern(a: Color, b: Color) -> Self {
        Pattern::Stripe(StripePattern::new(a, b))
    }
    pub fn stripe_at_object(&self, object: &Shape, world_point: Point) -> Color {
        let object_point = match object.get_inverse_transform() {
            None => world_point,
            Some(inverse_transform) => inverse_transform * world_point,
        };
        let pattern_point = match self.get_inverse_transform() {
            None => object_point,
            Some(inverse_transform) => inverse_transform * object_point,
        };
        self.stripe_at(pattern_point)
    }
    pub fn stripe_at(&self, point: Point) -> Color {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.color(point),
        }
    }
    fn a(&self) -> Color {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.a,
        }
    }
    fn b(&self) -> Color {
        match self {
            Pattern::Stripe(stripe_pattern) => stripe_pattern.b,
        }
    }
}
mod tests {
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
            pattern.stripe_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.stripe_at(Point {
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
            pattern.stripe_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }),
            white
        );
        assert_eq!(
            pattern.stripe_at(Point {
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
            pattern.stripe_at(Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: 0.9,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: -0.1,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            }),
            black
        );
        assert_eq!(
            pattern.stripe_at(Point {
                x: -1.1,
                y: 0.0,
                z: 0.0,
            }),
            white
        );
    }
}
