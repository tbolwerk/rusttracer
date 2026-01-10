use crate::intersections;
use crate::intersections::Computations;
use crate::intersections::Intersection;
use crate::intersections::Intersections;
use crate::lights;
use crate::lights::*;
use crate::materials::lightning;
use crate::materials::Material;
use crate::matrices::Matrix;
use crate::rays::Ray;
use crate::shapes::*;
use crate::transformations::*;
use crate::tuples::*;
use crate::patterns::*;

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub objects: Vec<Shape>,
    pub light: Option<Light>,
}
impl World {
    pub fn new() -> Self {
        Self {
            objects: vec![],
            light: None,
        }
    }
    pub fn intersect_world(&self, ray: &Ray) -> Intersections {
        let mut intersections = Intersections {
            intersections: vec![],
        };
        for (index, object) in self.objects.iter().enumerate() {
            let xs = object.intersect(&ray, index);
            intersections.extend(xs);
        }
        intersections
    }
    pub fn shade_hit(&self, comps: Computations, remaining: usize) -> Color {
        let object = &self.objects[comps.object_id];
        let shadowed = self.is_shadowed(comps.over_point);
        let surface = match self.light.clone() {
            None => Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            Some(light) => lightning(
                &object,
                light,
                comps.point,
                comps.eyev,
                comps.normalv,
                shadowed,
            ),
        };
        let reflected = self.reflected_color(&comps, remaining);
        let refracted = self.refracted_color(&comps, remaining);
        
        let material = object.get_material();
        if material.reflective > 0.0 && material.transparency > 0.0 {
            let reflectance = comps.schlick();
            return surface + reflected * reflectance + refracted * (1.0 - reflectance);
        }
        surface + reflected + refracted
    }
    pub fn color_at(&self, ray: &Ray, remaining: usize) -> Color {
        let xs = self.intersect_world(&ray);
        match xs.hit() {
            None => Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            Some(intersection) => self.shade_hit(intersection.prepare_computations(&ray, self, &xs), remaining),
        }
    }
    pub fn is_shadowed(&self, point: Point) -> bool {
        match self.light.clone() {
            None => true,
            Some(light) => {
                let v = light.position() - point;
                let distance = v.magnitude();
                let direction = v.normalize();

                let r = Ray {
                    origin: point,
                    direction,
                };

                let intersections = self.intersect_world(&r);

                match intersections.hit() {
                    None => false,
                    Some(intersection) => intersection.t < distance,
                }
            }
        }
    }
    pub fn reflected_color(&self, comps: &Computations, remaining: usize) -> Color {
        if remaining <= 0 {
            return Color {
                r:0.0,
                g:0.0,
                b:0.0
            }
        }
        let material = self.objects[comps.object_id].get_material();
        if material.reflective == 0.0 {

        return Color {r:0.0, g:0.0, b:0.0};
        }
        let reflect_ray = Ray {
            origin: comps.over_point,
            direction: comps.reflectv,
        };
        let color = self.color_at(&reflect_ray, remaining - 1);
        color * material.reflective
    }
    pub fn refracted_color(&self, comps: &Computations, remaining: usize) -> Color {
        let object = &self.objects[comps.object_id];
        if object.get_material().transparency == 0.0 || remaining <= 0 {
            return Color {r:0.0,g:0.0,b:0.0};
        }
        let n_ratio = comps.n1 / comps.n2;
        let cos_i = comps.eyev.dot(comps.normalv);
        let sin2_t = n_ratio.powi(2) * (1.0 - cos_i.powi(2));
        if sin2_t > 1.0 {
             return Color {r:0.0,g:0.0,b:0.0};
        }
        let cos_t = (1.0 - sin2_t).sqrt();
        let direction = comps.normalv * (n_ratio * cos_i - cos_t) - comps.eyev * n_ratio;
        let refract_ray = Ray {
            origin: comps.under_point,
            direction
        };
        self.color_at(&refract_ray, remaining -1) * object.get_material().transparency
    }
}
impl Default for World {
    fn default() -> Self {
        let light = Light::Point(PointLight {
            position: Point {
                x: -10.0,
                y: 10.0,
                z: -10.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        });
        let mut s1 = Shape::sphere();
        let mut m1: Material = Material::default();
        m1.set_color(Color {
            r: 0.8,
            g: 1.0,
            b: 0.6,
        });
        m1.set_diffuse(0.7);
        m1.set_specular(0.2);
        s1.set_material(m1);

        let mut s2 = Shape::sphere();
        const TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5);
        s2.set_transform(TRANSFORM);

        World {
            objects: vec![s1, s2],
            light: Some(light),
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn creating_a_world() {
        let w = World::new();
        assert_eq!(w.objects, vec![]);
        assert_eq!(w.light, None);
    }
    #[test]
    fn the_default_world() {
        let light = Light::Point(PointLight {
            position: Point {
                x: -10.0,
                y: 10.0,
                z: -10.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        });
        let mut s1 = Shape::sphere();
        let mut m1 = Material::default();
        m1.set_color(Color {
            r: 0.8,
            g: 1.0,
            b: 0.6,
        });
        m1.set_diffuse(0.7);
        m1.set_specular(0.2);
        s1.set_material(m1);

        let mut s2 = Shape::sphere();
        const TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5);
        s2.set_transform(TRANSFORM);

        let w = World::default();
        assert_eq!(w.light, Some(light));
        assert_eq!(w.objects[0], s1);
        assert_eq!(w.objects[1], s2);
    }
    #[test]
    fn intersect_a_world_with_a_ray() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = w.intersect_world(&r);
        assert_eq!(xs.count(), 4);
        assert_eq!(xs[0].t, 4.0);
        assert_eq!(xs[1].t, 4.5);
        assert_eq!(xs[2].t, 5.5);
        assert_eq!(xs[3].t, 6.0);
    }
    #[test]
    fn shading_an_intersection() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let i = Intersection::new(4.0, 0);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(
            w.shade_hit(comps, 0),
            Color {
                r: 0.38066,
                g: 0.47583,
                b: 0.2855
            }
        );
    }
    #[test]
    fn shading_an_intersection_from_the_inside() {
        let mut w = World::default();
        w.light = Some(Light::Point(PointLight {
            position: Point {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
            intensity: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
        }));

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let i = Intersection::new(0.5, 1);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(
            w.shade_hit(comps, 0),
            Color {
                r: 0.90498,
                g: 0.90498,
                b: 0.90498
            }
        );
    }
    #[test]
    fn the_color_when_a_ray_misses() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let c = w.color_at(&r, 0);
        assert_eq!(
            c,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }
    #[test]
    fn the_color_when_a_ray_hits() {
        let w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let c = w.color_at(&r, 0);
        assert_eq!(
            c,
            Color {
                r: 0.38066,
                g: 0.47583,
                b: 0.2855
            }
        );
    }
    #[test]
    fn the_color_with_an_intersection_behind_the_ray() {
        let mut w = World::default();
        let mut object_material0 = w.objects[0].get_material();
        object_material0.set_ambient(1.0);
        w.objects[0].set_material(object_material0);
        let mut object_material1 = w.objects[1].get_material();
        object_material1.set_ambient(1.0);
        w.objects[1].set_material(object_material1);

        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.75,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        };
        let c = w.color_at(&r, 0);
        assert_eq!(c, w.objects[1].get_material().color);
    }
    #[test]
    fn the_transformation_matrix_for_the_default_orientation() {
        let from = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Point {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        };
        let up = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(t, Matrix::identity());
    }
    #[test]
    fn a_view_transformation_matrix_looking_in_positive_z_direction() {
        let from = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Point {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let up = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(t, scaling(-1.0, 1.0, -1.0));
    }
    #[test]
    fn the_view_transformation_moves_the_world() {
        let from = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let to = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let up = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(t, scaling(0.0, 0.0, -8.0));
    }
    #[test]
    fn an_arbitrary_view_transformation() {
        let from = Point {
            x: 1.0,
            y: 3.0,
            z: 2.0,
        };
        let to = Point {
            x: 4.0,
            y: -2.0,
            z: 8.0,
        };
        let up = Vector {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        };
        let t = view_transform(from, to, up);
        assert_eq!(
            t,
            Matrix::new([
                [-0.50709, 0.50709, 0.67612, -2.36643],
                [0.76772, 0.60609, 0.12122, -2.82843],
                [-0.35857, 0.59761, -0.71714, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ])
        );
    }
    #[test]
    fn there_is_no_shadow_when_nothing_is_collinear_with_point_and_light() {
        let w = World::default();
        let p = Point {
            x: 0.0,
            y: 10.0,
            z: 0.0,
        };
        assert_eq!(w.is_shadowed(p), false);
    }
    #[test]
    fn the_shadow_when_an_object_is_between_the_point_and_the_light() {
        let w = World::default();
        let p = Point {
            x: 10.0,
            y: -10.0,
            z: 10.0,
        };
        assert_eq!(w.is_shadowed(p), true);
    }
    #[test]
    fn there_is_no_shadow_when_an_object_is_behind_the_light() {
        let w = World::default();
        let p = Point {
            x: -20.0,
            y: 20.0,
            z: -20.0,
        };
        assert_eq!(w.is_shadowed(p), false);
    }
    #[test]
    fn there_is_no_shadow_when_an_object_is_behind_the_point() {
        let w = World::default();
        let p = Point {
            x: -2.0,
            y: 2.0,
            z: -2.0,
        };
        assert_eq!(w.is_shadowed(p), false);
    }
    #[test]
    fn shade_hit_is_given_an_intersection_in_shadow() {
        let mut w = World::default();
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
        w.light = Some(light);
        let s1 = Shape::sphere();
        const TRANSFORM: Matrix<4, 4> = translation(0.0, 0.0, 10.0);
        let mut s2 = Shape::sphere();
        s2.set_transform(TRANSFORM);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let i = Intersection {
            t: 4.0,
            object_id: 1,
        };
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        w.objects.extend(vec![s1, s2.clone()]);
        let c = w.shade_hit(comps, 0);
        assert_eq!(
            c,
            Color {
                r: 0.1,
                g: 0.1,
                b: 0.1
            }
        );
    }
    #[test]
    fn the_hit_should_offset_the_point() {
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let mut shape = Shape::sphere();
        const TRANSFORM: Matrix<4, 4> = translation(0.0, 0.0, 1.0);
        shape.set_transform(TRANSFORM);
        let i = Intersection {
            t: 5.0,
            object_id: 0,
        };
        let mut w = World::new();
        w.objects.append(&mut vec![shape]);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(comps.over_point.z() < -EPSILON / 2.0, true);
        assert_eq!(comps.point.z() > comps.over_point.z(), true);
    }
    #[test]
    fn the_reflected_color_for_a_nonreflective_material() {
        let mut w = World::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y:0.0,
                z:0.0
            }, 
            direction: Vector { 
                x:0.0,
                y:0.0,
                z:1.0
            }
        };
        let mut second_object_material = w.objects[1].get_material();
        second_object_material.set_ambient(1.0);
        w.objects[1].set_material(second_object_material);
        let i = Intersection::new(1.0, 1);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        let color = w.reflected_color(&comps, 0);
        assert_eq!(color, Color{r:0.0, g:0.0,b:0.0});
    }
    #[test]
    fn the_reflected_color_for_a_reflective_material() {
        let mut w = World::default();
        let mut shape = Shape::plane();
        let mut material = Material::default();
        material.set_reflective(0.5);
        const TRANSFORM: Matrix<4,4> = Matrix::identity().then(translation(0.0,-1.0,0.0));
        shape.set_material(material);
        shape.set_transform(TRANSFORM);
        w.objects.append(&mut vec![shape]);

        let r = Ray {
            origin: Point {
                x: 0.0,
                y:0.0,
                z:-3.0
            },
            direction: Vector { 
                x:0.0,
                y:-sqrt(2.0)/2.0,
                z:sqrt(2.0)/2.0
            }
        };

        let i = Intersection::new(sqrt(2.0), 2);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        let color = w.reflected_color(&comps, 1);
        assert_eq!(color, Color{r:0.19032, g:0.2379,b:0.14274});
    }
    #[test]
    fn shade_hit_with_a_reflective_material() {
        let mut w = World::default();
        let mut shape = Shape::plane();
        let mut material = shape.get_material().clone();
        material.set_reflective(0.5);
        shape.set_material(material);
        shape.set_transform(translation(0.0,-1.0,0.0));
        w.objects.append(&mut vec![shape]);
        let r = Ray {
            origin: Point {x: 0.0, y:0.0, z:-3.0},
            direction: Vector {x:0.0, y:-sqrt(2.0)/2.0,z:sqrt(2.0)/2.0}
        };
        let i = Intersection::new(sqrt(2.0), 2);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        let color = w.shade_hit(comps, 1);
        assert_eq!(color, Color {
            r: 0.87677,
            g: 0.92436,
            b:0.82918
        })
    }
    #[test]
    fn color_at_with_mutally_reflective_surfaces() {
        let mut w = World::default();
        w.light = Some(Light::point_light(Point {x:0.0, y:0.0, z:0.0}, Color {r:1.0,g:1.0,b:1.0}));
        
        let mut lower = Shape::plane();
        let mut lower_material = lower.get_material().clone();
        lower_material.set_reflective(1.0);
        lower.set_transform(translation(0.0,-1.0,0.0));
        lower.set_material(lower_material);
        let mut upper = Shape::plane();
        let mut upper_material = upper.get_material().clone();
        upper_material.set_reflective(1.0);
        upper.set_transform(translation(0.0,1.0,0.0));
        upper.set_material(upper_material);
        w.objects.append(&mut vec![lower, upper]);
        let r = Ray {
            origin: Point {
                x: 0.0,y:0.0,z:0.0
            },
            direction: Vector {
                x:0.0,y:1.0,z:0.0
            }
        };
        let color = w.color_at(&r, 5);
        assert_eq!(color, Color {
            r:1.9,g:1.9,b:1.9
        })
    }
    #[test]
    fn the_refracted_color_with_an_opaque_surface() {
        let w = World::default();
        let shape = w.objects[0].clone();
        let r = Ray {
            origin: Point {
                x:0.0,y:0.0,z:-5.0
            },
            direction: Vector {
                x: 0.0,y:0.0,z:1.0
            }
        };
        let xs = Intersections::new(vec![Intersection::new(4.0, 0), Intersection::new(6.0, 0)]);
        let comps = xs[0].prepare_computations(&r, &w, &xs);
        let c = w.refracted_color(&comps, 5);
        assert_eq!(c, Color {r:0.0, g:0.0, b:0.0});
    }
    #[test]
    fn the_refracted_color_at_the_maximum_recursive_depth() {
        let mut w = World::default();
        w.objects[0] = Shape::glass_sphere(); 

        let r = Ray {
            origin: Point {
                x:0.0,y:0.0,z:-5.0
            },
            direction: Vector {
                x: 0.0,y:0.0,z:1.0
            }
        };
        let xs = Intersections::new(vec![Intersection::new(4.0, 0), Intersection::new(6.0, 0)]);
        let comps = xs[0].prepare_computations(&r, &w, &xs);
        let c = w.refracted_color(&comps, 0);
        assert_eq!(c, Color {r:0.0, g:0.0, b:0.0});
    }
  #[test]
    fn the_refracted_color_under_total_internal_reflection() {
        let mut w = World::default();
        w.objects[0] = Shape::glass_sphere(); 

        let r = Ray {
            origin: Point {
                x:0.0,y:0.0,z:sqrt(2.0)/2.0
            },
            direction: Vector {
                x: 0.0,y:1.0,z:0.0
            }
        };
        let xs = Intersections::new(vec![Intersection::new(-sqrt(2.0)/2.0, 0), Intersection::new(sqrt(2.0)/2.0, 0)]);
        let comps = xs[1].prepare_computations(&r, &w, &xs);
        let c = w.refracted_color(&comps, 5);
        assert_eq!(c, Color {r:0.0, g:0.0, b:0.0});
    }

  #[test]
    fn the_refracted_color_with_a_refracted_ray() {
        let mut w = World::default();
        let mut a_material = Material::default();
        a_material.set_ambient(1.0);
        a_material.set_pattern(Pattern::test_pattern());

        let a = Shape::with(
            Shape::sphere,
            Matrix::identity(),
            a_material
        );

        w.objects[0] = a;
        w.objects[1] = Shape::glass_sphere();

        let r = Ray {
            origin: Point {
                x:0.0,y:0.0,z:0.1
            },
            direction: Vector {
                x: 0.0,y:1.0,z:0.0
            }
        };
        let xs = Intersections::new(vec![Intersection::new(-0.9899, 0), Intersection::new(-0.4899, 1), Intersection::new(0.4800,1), Intersection::new(0.9899,0)]);
        let comps = xs[2].prepare_computations(&r, &w, &xs);
        let c = w.refracted_color(&comps, 5);
        assert_eq!(c, Color {r:0.0, g:0.99888, b:0.04725});
    }
    #[test]
    fn shade_hit_with_a_transparent_material() {
        let mut w = World::default();
        let mut glass = Material::default();
        glass.set_transparency(0.5);
        glass.set_refractive_index(1.5);

        let floor = Shape::with(Shape::plane, translation(0.0,-1.0,0.0), glass);
        let mut ball_material = Material::default();
        ball_material.set_color(Color {
            r:1.0,g:0.0,b:0.0
        });
        ball_material.set_ambient(0.5);
        let ball = Shape::with(Shape::sphere, translation(0.0, -3.5, -0.5), ball_material);
        w.objects.append(&mut vec![floor, ball]);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -3.0
            },
            direction: Vector {
                x:0.0,
                y:-sqrt(2.0)/2.0,
                z:sqrt(2.0)/2.0,
            }
        };
        let xs = Intersections::new(vec![Intersection::new(sqrt(2.0),2)]);
        let comps = xs[0].prepare_computations(&r, &w, &xs);
        let color = w.shade_hit(comps, 5);
        assert_eq!(color, Color {
            r: 0.93642,
            g: 0.68642,
            b: 0.68642
        })
    }
  #[test]
    fn shade_hit_with_a_reflective_transparent_material() {
        let mut w = World::default();
        let mut glass = Material::default();
        glass.set_transparency(0.5);
        glass.set_reflective(0.5);
        glass.set_refractive_index(1.5);

        let floor = Shape::with(Shape::plane, translation(0.0,-1.0,0.0), glass);
        let mut ball_material = Material::default();
        ball_material.set_color(Color {
            r:1.0,g:0.0,b:0.0
        });
        ball_material.set_ambient(0.5);
        let ball = Shape::with(Shape::sphere, translation(0.0, -3.5, -0.5), ball_material);
        w.objects.append(&mut vec![floor, ball]);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -3.0
            },
            direction: Vector {
                x:0.0,
                y:-sqrt(2.0)/2.0,
                z:sqrt(2.0)/2.0,
            }
        };
        let xs = Intersections::new(vec![Intersection::new(sqrt(2.0),2)]);
        let comps = xs[0].prepare_computations(&r, &w, &xs);
        let color = w.shade_hit(comps, 5);
        assert_eq!(color, Color {
            r: 0.93391,
            g: 0.69643,
            b: 0.69243
        })
    }


}
