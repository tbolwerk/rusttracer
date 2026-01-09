use crate::{
    lights::*,
    patterns::Pattern,
    shapes::{HasMaterial, Shape},
    tuples::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub color: Color,
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub shininess: f32,
    pub pattern: Option<Pattern>,
    pub reflective: f32,
    pub transparency: f32,
    pub refractive_index: f32,
}

impl Material {
    pub const fn new(
        color: Color,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
        reflective: f32,
        transparency: f32,
        refractive_index: f32,
    ) -> Self {
        Self {
            color,
            ambient,
            diffuse,
            specular,
            shininess,
            pattern: None,
            reflective,
            transparency,
            refractive_index,
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
            0.0,
            0.0,
            1.0,
        )
    }
    pub const fn glass() -> Self {
        Self {
            color: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            ambient: 1.0,
            diffuse: 1.0,
            specular: 1.0,
            shininess: 300.0,
            pattern: None,
            reflective: 0.9,
            transparency: 0.0,
            refractive_index: 0.5,
        }
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
    pub const fn set_pattern(&mut self, pattern: Pattern) -> () {
        self.pattern = Some(pattern)
    }
    pub const fn set_reflective(&mut self, reflective: f32) -> () {
        self.reflective = reflective
    }
    pub const fn set_transparency(&mut self, transparency: f32) -> () {
        self.transparency = transparency
    }
    pub const fn set_refractive_index(&mut self, refractive_index: f32) -> () {
        self.refractive_index = refractive_index
    }
}

pub fn lightning(
    object: &Shape,
    light: Light,
    point: Point,
    eyev: Vector,
    normalv: Vector,
    in_shadow: bool,
) -> Color {
    let material = object.get_material();
    let color = match material.pattern {
        None => material.color,
        Some(ref pattern) => pattern.pattern_at_shape(object, point),
    };
    let effective_color = color * light.intensity();
    let lightv = (light.position() - point).normalize();
    let ambient = effective_color * material.ambient;

    if in_shadow {
        return ambient;
    }

    let light_dot_normal = lightv.dot(normalv);
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
        let reflectv = (-lightv).reflect(normalv);
        let reflect_dot_eye = reflectv.dot(eyev);
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

mod tests {
    use crate::{
        intersections::{Intersection, Intersections},
        rays::Ray,
        worlds::World,
    };

    use super::*;

    #[test]
    fn lightning_with_the_eye_between_the_light_and_the_surface() {
        let (m, position) = background();
        let mut object = Shape::sphere();
        object.set_material(m);

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
        let result = lightning(&object, light, position, eyev, normalv, false);
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
        let mut object = Shape::sphere();
        object.set_material(m);

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
        let result = lightning(&object, light, position, eyev, normalv, false);
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
        let mut object = Shape::sphere();
        object.set_material(m);

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
        let result = lightning(&object, light, position, eyev, normalv, false);
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
        let mut object = Shape::sphere();
        object.set_material(m);

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
        let result = lightning(&object, light, position, eyev, normalv, false);
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
        let mut object = Shape::sphere();
        object.set_material(m);

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
        let result = lightning(&object, light, position, eyev, normalv, false);
        assert_eq!(
            result,
            Color {
                r: 0.1,
                g: 0.1,
                b: 0.1
            }
        );
    }
    #[test]
    fn lighting_with_the_surface_in_shadow() {
        let (m, position) = background();
        let mut object = Shape::sphere();
        object.set_material(m);

        let eyev = Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
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
        let in_shadow = true;
        let result = lightning(&object, light, position, eyev, normalv, in_shadow);
        assert_eq!(
            Color {
                r: 0.1,
                g: 0.1,
                b: 0.1
            },
            result
        );
    }
    #[test]
    fn lighting_with_a_pattern_applied() {
        let (m, _) = background();
        let mut material = m.clone();
        material.set_pattern(Pattern::stripe_pattern(
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
        ));
        material.set_ambient(1.0);
        material.set_diffuse(0.0);
        material.set_specular(0.0);
        let mut object = Shape::sphere();
        object.set_material(material);
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
        let light = Light::point_light(
            Point {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        );
        let c1 = lightning(
            &object,
            light.clone(),
            Point {
                x: 0.9,
                y: 0.0,
                z: 0.0,
            },
            eyev,
            normalv,
            false,
        );
        let c2 = lightning(
            &object,
            light,
            Point {
                x: 1.1,
                y: 0.0,
                z: 0.0,
            },
            eyev,
            normalv,
            false,
        );
        assert_eq!(
            c1,
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0
            }
        );
        assert_eq!(
            c2,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }
    #[test]
    fn reflectivity_for_the_default_material() {
        let m = Material::default();
        assert_eq!(m.reflective, 0.0);
    }
    #[test]
    fn precomputing_the_reflection_vector() {
        let mut w = World::new();
        let shape = Shape::plane();
        w.objects.append(&mut vec![shape]);

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 1.0,
                z: -1.0,
            },
            direction: Vector {
                x: 0.0,
                y: -(2.0_f32.sqrt() / 2.0),
                z: 2.0_f32.sqrt() / 2.0,
            },
        };
        let i = Intersection::new(2.0_f32.sqrt(), 0);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(
            comps.reflectv,
            Vector {
                x: 0.0,
                y: 2.0_f32.sqrt() / 2.0,
                z: 2.0_f32.sqrt() / 2.0
            }
        )
    }
    #[test]
    fn transparency_and_refractive_index_for_the_default_material() {
        let mut m = Material::default();
        m.set_transparency(0.0);
        m.set_refractive_index(1.0);
        assert_eq!(m.transparency, 0.0);
        assert_eq!(m.refractive_index, 1.0);
    }
}
