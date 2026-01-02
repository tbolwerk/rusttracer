use crate::{canvas::*, external_tuples::*, lights::*, tuples::*};

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Material {
    color: TupleKind,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    shininess: f32,
}

impl Material {
    pub const fn new(
        color: TupleKind,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    ) -> Self {
        Self {
            color,
            ambient,
            diffuse,
            specular,
            shininess,
        }
    }
    pub const fn default() -> Self {
        Self::new(TupleKind::color(1.0, 1.0, 1.0), 0.1, 0.9, 0.9, 200.0)
    }
    pub const fn set_color(&mut self, color: TupleKind) -> () {
        self.color = color
    }
    pub const fn set_ambient(&mut self, ambient: f32) -> () {
        self.ambient = ambient
    }
    pub const fn set_diffuse(&mut self, diffuse: f32) -> () {
        self.diffuse = diffuse
    }
    pub const fn set_specular(&mut self, specular: f32) -> () {
        self.specular = specular
    }
    pub const fn set_shininess(&mut self, shininess: f32) -> () {
        self.shininess = shininess
    }
}

pub fn lightning(
    material: &Material,
    light: Light,
    point: TupleKind,
    eyev: TupleKind,
    normalv: TupleKind,
) -> TupleKind {
    let effective_color = material.color * light.intensity();
    let lightv = (light.position() - point).normalize();
    let ambient = effective_color * material.ambient;

    let light_dot_normal = lightv.dot(&normalv);
    let mut diffuse = TupleKind::color(0.0, 0.0, 0.0);
    let mut specular = TupleKind::color(0.0, 0.0, 0.0);
    if light_dot_normal >= 0.0 {
        diffuse = effective_color * material.diffuse * light_dot_normal;
        let reflectv = (-lightv).reflect(&normalv);
        let reflect_dot_eye = reflectv.dot(&eyev);
        if reflect_dot_eye > 0.0 {
            let factor = reflect_dot_eye.powf(material.shininess);
            specular = light.intensity() * material.specular * factor;
        }
    }
    let result = ambient + diffuse + specular;
    TupleKind::point(result.x(), result.y(), result.z())
}
#[test]
fn the_default_meterial() {
    let m = Material::default();
    assert_eq!(m.color, TupleKind::point(1.0, 1.0, 1.0));
    assert_eq!(m.ambient, 0.1);
    assert_eq!(m.diffuse, 0.9);
    assert_eq!(m.specular, 0.9);
    assert_eq!(m.shininess, 200.0);
}

fn background() -> (Material, TupleKind) {
    let m = Material::default();
    let position = TupleKind::point(0.0, 0.0, 0.0);
    (m, position)
}

#[test]
fn lightning_with_the_eye_between_the_light_and_the_surface() {
    let (m, position) = background();
    let eyev = TupleKind::vector(0.0, 0.0, -1.0);
    let normalv = TupleKind::vector(0.0, 0.0, -1.0);
    let light = Light::Point(PointLight {
        position: TupleKind::point(0.0, 0.0, -10.0),
        intensity: TupleKind::point(1.0, 1.0, 1.0),
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(result, TupleKind::color(1.9, 1.9, 1.9));
}
#[test]
fn lightning_with_the_eye_between_the_light_and_the_surface_eye_offset_45_degrees() {
    let (m, position) = background();
    let eyev = TupleKind::vector(0.0, 2.0_f32.sqrt() / 2.0, 2.0_f32.sqrt() / 2.0);
    let normalv = TupleKind::vector(0.0, 0.0, -1.0);
    let light = Light::Point(PointLight {
        position: TupleKind::point(0.0, 0.0, -10.0),
        intensity: TupleKind::color(1.0, 1.0, 1.0),
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(result, TupleKind::color(1.0, 1.0, 1.0));
}
#[test]
fn lightning_with_eye_opposite_surface_light_offset_45_degrees() {
    let (m, position) = background();
    let eyev = TupleKind::vector(0.0, 0.0, -1.0);
    let normalv = TupleKind::vector(0.0, 0.0, -1.0);
    let light = Light::Point(PointLight {
        position: TupleKind::point(0.0, 10.0, -10.0),
        intensity: TupleKind::color(1.0, 1.0, 1.0),
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(result, TupleKind::color(0.7364, 0.7364, 0.7364));
}
#[test]
fn lightning_with_eye_in_the_path_of_the_reflection_vector() {
    let (m, position) = background();
    let eyev = TupleKind::vector(0.0, -(2.0_f32.sqrt() / 2.0), -(2.0_f32.sqrt() / 2.0));
    let normalv = TupleKind::vector(0.0, 0.0, -1.0);
    let light = Light::Point(PointLight {
        position: TupleKind::point(0.0, 10.0, -10.0),
        intensity: TupleKind::color(1.0, 1.0, 1.0),
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(result, TupleKind::point(1.6364, 1.6364, 1.6364));
}
#[test]
fn lightning_with_the_light_behind_the_surface() {
    let (m, position) = background();
    let eyev = TupleKind::vector(0.0, 0.0, -1.0);
    let normalv = TupleKind::vector(0.0, 0.0, -1.0);
    let light = Light::Point(PointLight {
        position: TupleKind::point(0.0, 0.0, 10.0),
        intensity: TupleKind::color(1.0, 1.0, 1.0),
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(result, TupleKind::color(0.1, 0.1, 0.1));
}
