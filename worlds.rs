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
use crate::spheres::*;
use crate::transformations::*;
use crate::tuples::*;

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    objects: Vec<Sphere>,
    light: Option<Light>,
}
impl World {
    pub fn new() -> Self {
        Self {
            objects: vec![],
            light: None,
        }
    }
    pub fn intersect_world(&self, ray: &Ray) -> Intersections<'_> {
        let mut intersections = Intersections {
            intersections: vec![],
        };
        for object in &self.objects {
            let xs = object.intersect(&ray);
            intersections.extend(xs);
        }
        intersections
    }
    pub fn shade_hit(&self, comps: Computations) -> Color {
        match self.light.clone() {
            None => Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            Some(light) => lightning(
                &comps.object.material,
                light,
                comps.point,
                comps.eyev,
                comps.normalv,
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
            Some(intersections) => self.shade_hit(intersections.prepare_computations(&ray)),
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
        let mut s1 = Sphere::unit();
        let mut m1 = Material::default();
        m1.set_color(Color {
            r: 0.8,
            g: 1.0,
            b: 0.6,
        });
        m1.set_diffuse(0.7);
        m1.set_specular(0.2);
        s1.set_material(&m1);

        let mut s2 = Sphere::unit();
        const TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5);
        s2.set_transform(&TRANSFORM);

        World {
            objects: vec![s1, s2],
            light: Some(light),
        }
    }
}

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
    let mut s1 = Sphere::unit();
    let mut m1 = Material::default();
    m1.set_color(Color {
        r: 0.8,
        g: 1.0,
        b: 0.6,
    });
    m1.set_diffuse(0.7);
    m1.set_specular(0.2);
    s1.set_material(&m1);

    let mut s2 = Sphere::unit();
    const TRANSFORM: Matrix<4, 4> = scaling(0.5, 0.5, 0.5);
    s2.set_transform(&TRANSFORM);

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
    let shape = w.objects[0].clone();
    let i = Intersection::new(4.0, &shape);
    let comps = i.prepare_computations(&&r);
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
    let shape = w.objects[1].clone();
    let i = Intersection::new(0.5, &shape);
    let comps = i.prepare_computations(&&r);
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
    w.objects[0].material.set_ambient(1.0);
    w.objects[1].material.set_ambient(1.0);

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
    assert_eq!(c, w.objects[1].material.color);
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
