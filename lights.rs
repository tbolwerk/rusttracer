use crate::tuples::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PointLight {
    pub position: Point,
    pub intensity: Color,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Light {
    Point(PointLight),
}

impl PointLight {
    pub const fn new(position: Point, intensity: Color) -> Self {
        Self {
            position,
            intensity,
        }
    }
}

pub trait LightProperties {
    fn position(&self) -> Point;
    fn intensity(&self) -> Color;
}

impl LightProperties for PointLight {
    fn position(&self) -> Point {
        self.position.clone()
    }
    fn intensity(&self) -> Color {
        self.intensity.clone()
    }
}

impl LightProperties for Light {
    fn position(&self) -> Point {
        match self {
            Light::Point(light) => light.position.clone(),
        }
    }
    fn intensity(&self) -> Color {
        match self {
            Light::Point(light) => light.intensity.clone(),
        }
    }
}

#[test]
fn a_point_light_has_a_position_and_intensity() {
    let intensity = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    let position = Point {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let light = Light::Point(PointLight {
        position: position.clone(),
        intensity: intensity.clone(),
    });
    assert_eq!(light.position(), position);
    assert_eq!(light.intensity(), intensity);
}
