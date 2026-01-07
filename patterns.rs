use crate::tuples::*;

#[derive(PartialEq, Debug, Clone)]
struct StripePattern {
    a: Color,
    b: Color,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Pattern {
    Stripe(StripePattern),
}

impl StripePattern {
    fn color(&self, point: Point) -> Color {
        if point.x().floor() % 2.0 == 0.0 {
            return self.a;
        }
        self.b
    }
}

impl Pattern {
    pub fn stripe_pattern(a: Color, b: Color) -> Self {
        Pattern::Stripe(StripePattern { a, b })
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
