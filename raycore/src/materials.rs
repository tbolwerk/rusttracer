use crate::{
    lights::*,
    patterns::Pattern,
    shapes::{HasMaterial, Primitive},
    tuples::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub color: Color,
    pub ambient: Number,
    pub diffuse: Number,
    pub specular: Number,
    pub shininess: Number,
    pub pattern: Pattern,
    pub reflective: Number,
    pub transparency: Number,
    pub refractive_index: Number,
}

impl Material {
    pub const fn new(
        color: Color,
        ambient: Number,
        diffuse: Number,
        specular: Number,
        shininess: Number,
        reflective: Number,
        transparency: Number,
        refractive_index: Number,
    ) -> Self {
        Self {
            color,
            ambient,
            diffuse,
            specular,
            shininess,
            pattern: Pattern::none(),
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
            ambient: 0.0,
            diffuse: 0.0,
            specular: 1.0,
            shininess: 300.0,
            pattern: Pattern::none(),
            reflective: 0.1,
            transparency: 1.0,
            refractive_index: 1.5,
        }
    }
    pub const fn set_color(&mut self, color: Color) -> () {
        self.color = color
    }
    pub const fn set_ambient(&mut self, ambient: Number) -> () {
        self.ambient = ambient
    }
    pub const fn set_diffuse(&mut self, diffuse: Number) -> () {
        self.diffuse = diffuse
    }
    pub const fn set_specular(&mut self, specular: Number) -> () {
        self.specular = specular
    }
    pub const fn set_shininess(&mut self, shininess: Number) -> () {
        self.shininess = shininess
    }
    pub const fn set_pattern(&mut self, pattern: Pattern) -> () {
        self.pattern = pattern
    }
    pub const fn set_reflective(&mut self, reflective: Number) -> () {
        self.reflective = reflective
    }
    pub const fn set_transparency(&mut self, transparency: Number) -> () {
        self.transparency = transparency
    }
    pub const fn set_refractive_index(&mut self, refractive_index: Number) -> () {
        self.refractive_index = refractive_index
    }
}

// `intensity` is the fraction of the light visible from `point` (1.0 fully lit,
// 0.0 fully shadowed, in between for an area light's penumbra), as returned by
// `World::intensity_at`. Diffuse and specular are summed over every sample point
// on the light's surface and averaged, so an area light also softens highlights,
// then scaled by `intensity`. A point light is a single sample, recovering the
// original Phong result with `intensity` standing in for the old shadow flag.
pub fn lightning(
    object: &Primitive,
    light: Light,
    point: Point,
    eyev: Vector,
    normalv: Vector,
    intensity: Number,
) -> Color {
    let material = object.get_material();
    let color = if material.pattern.kind != 0 {
        material.pattern.pattern_at_shape(object, point)
    } else {
        material.color
    };
    let effective_color = color * light.intensity();
    let ambient = effective_color * material.ambient;

    let black = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    let mut diffuse_sum = black;
    let mut specular_sum = black;
    for v in 0..light.vsteps() {
        for u in 0..light.usteps() {
            let lightv = (light.point_on_light(u, v) - point).normalize();
            let light_dot_normal = lightv.dot(normalv);
            if light_dot_normal >= 0.0 {
                diffuse_sum = diffuse_sum + effective_color * material.diffuse * light_dot_normal;
                let reflectv = (-lightv).reflect(normalv);
                let reflect_dot_eye = reflectv.dot(eyev);
                if reflect_dot_eye > 0.0 {
                    let factor = reflect_dot_eye.powf(material.shininess);
                    specular_sum = specular_sum + light.intensity() * material.specular * factor;
                }
            }
        }
    }
    let samples = light.samples() as Number;
    let diffuse = diffuse_sum * (1.0 / samples);
    let specular = specular_sum * (1.0 / samples);
    ambient + (diffuse + specular) * intensity
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

#[cfg(test)]
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
        let mut object = Primitive::sphere();
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
        let light = Light::point_light(Point {
                x: 0.0,
                y: 0.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let result = lightning(&object, light, position, eyev, normalv, 1.0);
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
        let mut object = Primitive::sphere();
        object.set_material(m);

        let eyev = Vector {
            x: 0.0,
            y: sqrt(2.0) / 2.0,
            z: sqrt(2.0) / 2.0,
        };
        let normalv = Vector {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        let light = Light::point_light(Point {
                x: 0.0,
                y: 0.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let result = lightning(&object, light, position, eyev, normalv, 1.0);
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
        let mut object = Primitive::sphere();
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
        let light = Light::point_light(Point {
                x: 0.0,
                y: 10.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let result = lightning(&object, light, position, eyev, normalv, 1.0);
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
        let mut object = Primitive::sphere();
        object.set_material(m);

        let eyev = Vector {
            x: 0.0,
            y: -(sqrt(2.0) / 2.0),
            z: -(sqrt(2.0) / 2.0),
        };
        let normalv = Vector {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        let light = Light::point_light(Point {
                x: 0.0,
                y: 10.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let result = lightning(&object, light, position, eyev, normalv, 1.0);
        // Looser tolerance: the f32 specular term (powf(shininess)) drifts past EPSILON.
        let expected = 1.6364;
        assert!((result.r - expected).abs() < 1e-3);
        assert!((result.g - expected).abs() < 1e-3);
        assert!((result.b - expected).abs() < 1e-3);
    }
    #[test]
    fn lightning_with_the_light_behind_the_surface() {
        let (m, position) = background();
        let mut object = Primitive::sphere();
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
        let light = Light::point_light(Point {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let result = lightning(&object, light, position, eyev, normalv, 1.0);
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
        let mut object = Primitive::sphere();
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
        let light = Light::point_light(Point {
                x: 0.0,
                y: 0.0,
                z: -10.0,
            }, Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
        let in_shadow = 0.0;
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
        let mut object = Primitive::sphere();
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
            1.0,
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
            1.0,
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
        let shape = Primitive::plane();
        w.objects.append(&mut vec![shape]);

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 1.0,
                z: -1.0,
            },
            direction: Vector {
                x: 0.0,
                y: -(sqrt(2.0) / 2.0),
                z: sqrt(2.0) / 2.0,
            },
        };
        let i = Intersection::new(sqrt(2.0), 0);
        let comps = i.prepare_computations(&r, &w.scene(), &Intersections::new(vec![]));
        assert_eq!(
            comps.reflectv,
            Vector {
                x: 0.0,
                y: sqrt(2.0) / 2.0,
                z: sqrt(2.0) / 2.0
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
