use crate::{lights::*, tuples::*};

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub color: Color,
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub shininess: f32,
}

impl Material {
    pub const fn new(
        color: Color,
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
        Self::new(
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            0.1,
            0.9,
            0.9,
            200.0,
        )
    }
    pub const fn set_color(&mut self, color: Color) -> () {
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
    point: Point,
    eyev: Vector,
    normalv: Vector,
) -> Color {
    let effective_color = material.color.clone() * light.intensity();
    let lightv = (light.position() - point).normalize();
    let ambient = effective_color.clone() * material.ambient;

    let light_dot_normal = lightv.dot(&normalv);
    let mut diffuse = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    let mut specular = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    if light_dot_normal >= 0.0 {
        diffuse = effective_color * material.diffuse * light_dot_normal;
        let reflectv = (-lightv).reflect(&normalv);
        let reflect_dot_eye = reflectv.dot(&eyev);
        if reflect_dot_eye > 0.0 {
            let factor = reflect_dot_eye.powf(material.shininess);
            specular = light.intensity() * material.specular * factor;
        }
    }
    ambient + diffuse + specular
}
#[test]
fn the_default_meterial() {
    let m = Material::default();
    assert_eq!(
        m.color,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0
        }
    );
    assert_eq!(m.ambient, 0.1);
    assert_eq!(m.diffuse, 0.9);
    assert_eq!(m.specular, 0.9);
    assert_eq!(m.shininess, 200.0);
}

fn background() -> (Material, Point) {
    let m = Material::default();
    let position = Point {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    (m, position)
}

#[test]
fn lightning_with_the_eye_between_the_light_and_the_surface() {
    let (m, position) = background();
    let eyev = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let normalv = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let light = Light::Point(PointLight {
        position: Point {
            x: 0.0,
            y: 0.0,
            z: -10.0,
        },
        intensity: Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(
        result,
        Color {
            r: 1.9,
            g: 1.9,
            b: 1.9
        }
    );
}
#[test]
fn lightning_with_the_eye_between_the_light_and_the_surface_eye_offset_45_degrees() {
    let (m, position) = background();
    let eyev = Vector {
        x: 0.0,
        y: 2.0_f32.sqrt() / 2.0,
        z: 2.0_f32.sqrt() / 2.0,
    };
    let normalv = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let light = Light::Point(PointLight {
        position: Point {
            x: 0.0,
            y: 0.0,
            z: -10.0,
        },
        intensity: Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(
        result,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0
        }
    );
}
#[test]
fn lightning_with_eye_opposite_surface_light_offset_45_degrees() {
    let (m, position) = background();
    let eyev = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let normalv = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let light = Light::Point(PointLight {
        position: Point {
            x: 0.0,
            y: 10.0,
            z: -10.0,
        },
        intensity: Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(
        result,
        Color {
            r: 0.7364,
            g: 0.7364,
            b: 0.7364
        }
    );
}
#[test]
fn lightning_with_eye_in_the_path_of_the_reflection_vector() {
    let (m, position) = background();
    let eyev = Vector {
        x: 0.0,
        y: -(2.0_f32.sqrt() / 2.0),
        z: -(2.0_f32.sqrt() / 2.0),
    };
    let normalv = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let light = Light::Point(PointLight {
        position: Point {
            x: 0.0,
            y: 10.0,
            z: -10.0,
        },
        intensity: Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(
        result,
        Color {
            r: 1.6364,
            g: 1.6364,
            b: 1.6364
        }
    );
}
#[test]
fn lightning_with_the_light_behind_the_surface() {
    let (m, position) = background();
    let eyev = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let normalv = Vector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    let light = Light::Point(PointLight {
        position: Point {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        },
        intensity: Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
    });
    let result = lightning(&m, light, position, eyev, normalv);
    assert_eq!(
        result,
        Color {
            r: 0.1,
            g: 0.1,
            b: 0.1
        }
    );
}
