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
    pub fn shade_hit(&self, comps: Computations) -> Color {
        let object = &self.objects[comps.object_id];
        let shadowed = self.is_shadowed(comps.over_point);
        match self.light.clone() {
            None => Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            Some(light) => lightning(
                &object.get_material(),
                light,
                comps.point,
                comps.eyev,
                comps.normalv,
                shadowed,
            ),
        }
    }
    pub fn color_at(&self, ray: &Ray) -> Color {
        let xs = self.intersect_world(&ray);
        match xs.hit() {
            None => Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            Some(intersection) => self.shade_hit(intersection.prepare_computations(&ray, self)),
        }
    }
    pub fn is_shadowed(&self, point: Point) -> bool {
        match self.light.clone() {
            None => true,
            Some(light) => {
                let v = light.position() - point.clone();
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
        let comps = i.prepare_computations(&r, &w);
        assert_eq!(
            w.shade_hit(comps),
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
        let comps = i.prepare_computations(&r, &w);
        assert_eq!(
            w.shade_hit(comps),
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
        let c = w.color_at(&r);
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
        let c = w.color_at(&r);
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
        let c = w.color_at(&r);
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
        let comps = i.prepare_computations(&r, &w);
        w.objects.extend(vec![s1, s2.clone()]);
        let c = w.shade_hit(comps);
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
        let comps = i.prepare_computations(&r, &w);
        assert_eq!(comps.over_point.z() < -EPSILON / 2.0, true);
        assert_eq!(comps.point.z() > comps.over_point.z(), true);
    }
}
