use crate::tuples::external_tuples::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub position: TupleKind,
    pub intensity: TupleKind,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Light {
    Point(PointLight),
}

impl PointLight {
    pub const fn new(position: TupleKind, intensity: TupleKind) -> Self {
        Self {
            position,
            intensity,
        }
    }
}

pub trait LightProperties {
    fn position(&self) -> TupleKind;
    fn intensity(&self) -> TupleKind;
}

impl LightProperties for PointLight {
    fn position(&self) -> TupleKind {
        self.position
    }
    fn intensity(&self) -> TupleKind {
        self.intensity
    }
}

impl LightProperties for Light {
    fn position(&self) -> TupleKind {
        match self {
            Light::Point(light) => light.position,
        }
    }
    fn intensity(&self) -> TupleKind {
        match self {
            Light::Point(light) => light.intensity,
        }
    }
}

#[test]
fn a_point_light_has_a_position_and_intensity() {
    let intensity = TupleKind::color(1.0, 1.0, 1.0);
    let position = TupleKind::point(0.0, 0.0, 0.0);
    let light = Light::Point(PointLight {
        position,
        intensity,
    });
    assert_eq!(light.position(), position);
    assert_eq!(light.intensity(), intensity);
}
