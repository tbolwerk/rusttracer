use crate::intersections;
use crate::intersections::*;
use crate::materials::*;
use crate::matrices::*;
use crate::rays::*;
use crate::transformations::*;
use crate::tuples::mytuples::*;

#[derive(Debug, PartialEq, Clone)]
pub struct Sphere {
    pub origin: Point,
    pub radius: f32,
    pub transform: Matrix<4, 4>,
    pub inverse_transform: Option<Matrix<4, 4>>,
    pub material: Material,
}

impl Sphere {
    pub const fn new(
        origin: Point,
        radius: f32,
        transform: Matrix<4, 4>,
        material: Material,
    ) -> Self {
        Self {
            origin,
            radius,
            transform,
            inverse_transform: None,
            material,
        }
    }
    pub const fn unit() -> Self {
        Self::new(
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            1.0,
            Matrix::identity(),
            Material::default(),
        )
    }
    pub fn intersect(&self, ray: &Ray) -> Intersections<'_> {
        let ray2 = match self.inverse_transform {
            None => ray,
            Some(m) => &ray.transform(m),
        };

        let sphere_to_ray = ray2.origin.clone() - self.origin.clone();

        let a = ray2.direction.dot(&ray2.direction);
        let b = 2.0 * ray2.direction.dot(&sphere_to_ray);
        let c = sphere_to_ray.dot(&sphere_to_ray) - 1.0;

        let discriminant = b.powi(2) - 4.0 * a * c;

        if discriminant < 0.0 {
            return Intersections::new(vec![]);
        }

        let t1 = (-b - discriminant.sqrt()) / (2.0 * a);
        let t2 = (-b + discriminant.sqrt()) / (2.0 * a);

        let i1 = Intersection::new(t1, &self);
        let i2 = Intersection::new(t2, &self);
        Intersections::new(vec![i1, i2])
    }
    pub fn set_transform(&mut self, transform: &Matrix<4, 4>) -> () {
        self.transform = transform.clone();
        self.inverse_transform = inverse(&transform);
    }
    pub fn normal_at(&self, world_point: &Point) -> Vector {
        match self.inverse_transform {
            None => (world_point.clone() - self.origin.clone()).normalize(),
            Some(inv) => {
                let object_point = inv * world_point.clone();
                let object_normal = object_point - Point::default();
                let mut world_normal = transpose(&inv) * object_normal;
                world_normal.set(3, 0.0);
                world_normal.normalize()
            }
        }
    }
    pub fn set_material(&mut self, material: &Material) -> () {
        self.material = material.clone();
    }
}

#[test]
fn a_ray_intersects_a_sphere_at_two_points() {
    const R: Ray = Ray {
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
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, 4.0);
    assert_eq!(xs[1].t, 6.0);
}
#[test]
fn a_ray_intersects_a_sphere_at_a_tangent() {
    const R: Ray = Ray {
        origin: Point {
            x: 0.0,
            y: 1.0,
            z: -5.0,
        },
        direction: Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    };
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, 5.0);
    assert_eq!(xs[1].t, 5.0);
}
#[test]
fn a_ray_misses_a_sphere() {
    const R: Ray = Ray {
        origin: Point {
            x: 0.0,
            y: 2.0,
            z: -5.0,
        },
        direction: Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    };
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs.count(), 0);
}
#[test]
fn a_ray_originates_inside_a_sphere() {
    const R: Ray = Ray {
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
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, -1.0);
    assert_eq!(xs[1].t, 1.0);
}
#[test]
fn a_sphere_is_behind_a_ray() {
    const R: Ray = Ray {
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
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    assert_eq!(xs[0].t, -6.0);
    assert_eq!(xs[1].t, -4.0);
}
#[test]
fn intersect_sets_the_object_on_the_intersection() {
    const R: Ray = Ray {
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
    const S: Sphere = Sphere::unit();
    let xs = S.intersect(&R);
    Intersections::new(vec![xs[0], xs[1]]);
    assert_eq!(xs[0].object, &S);
}
#[test]
fn a_spheres_default_transformation() {
    const S: Sphere = Sphere::unit();
    assert_eq!(S.transform, Matrix::identity());
}
#[test]
fn changing_a_spheres_transformation() {
    let mut s: Sphere = Sphere::unit();
    const T: Matrix<4, 4> = translation(2.0, 3.0, 4.0);
    s.set_transform(&T);
    assert_eq!(s.transform, T);
}
#[test]
fn intersecting_a_scaled_sphere_with_a_ray() {
    const R: Ray = Ray {
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
    let mut s = Sphere::unit();
    s.set_transform(&scaling(2.0, 2.0, 2.0));
    let xs = s.intersect(&R);
    assert_eq!(xs.count(), 2);
    assert_eq!(xs[0].t, 3.0);
    assert_eq!(xs[1].t, 7.0);
}
#[test]
fn intersecting_a_translated_sphere_with_a_ray() {
    const R: Ray = Ray {
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
    let mut s = Sphere::unit();
    s.set_transform(&translation(5.0, 0.0, 0.0));
    let xs = s.intersect(&R);
    assert_eq!(xs.count(), 0);
}
#[test]
fn the_normal_on_a_sphere_at_a_point_on_the_x_axis() {
    let s = Sphere::unit();
    let n = s.normal_at(&Point {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    assert_eq!(
        n,
        Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0
        }
    );
}
#[test]
fn the_normal_on_a_sphere_at_a_point_on_the_y_axis() {
    let s = Sphere::unit();
    let n = s.normal_at(&Point {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    });
    assert_eq!(
        n,
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0
        }
    );
}
#[test]
fn the_normal_on_a_sphere_at_a_point_on_the_z_axis() {
    let s = Sphere::unit();
    let n = s.normal_at(&Point {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    assert_eq!(
        n,
        Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0
        }
    );
}
#[test]
fn the_normal_on_a_sphere_at_a_nonaxial_point() {
    let s = Sphere::unit();
    let n = s.normal_at(&Point {
        x: 3.0_f32.sqrt() / 3.0,
        y: 3.0_f32.sqrt() / 3.0,
        z: 3.0_f32.sqrt() / 3.0,
    });
    assert_eq!(
        n,
        Vector {
            x: 3.0_f32.sqrt() / 3.0,
            y: 3.0_f32.sqrt() / 3.0,
            z: 3.0_f32.sqrt() / 3.0,
        }
    );
}
#[test]
fn the_normal_is_a_normalized_vector() {
    let s = Sphere::unit();
    let n = s.normal_at(&Point {
        x: 3.0_f32.sqrt() / 3.0,
        y: 3.0_f32.sqrt() / 3.0,
        z: 3.0_f32.sqrt() / 3.0,
    });
    assert_eq!(n, n.normalize());
}
#[test]
fn computing_the_normal_on_a_translated_sphere() {
    let mut s = Sphere::unit();
    s.set_transform(&translation(0.0, 1.0, 0.0));
    let n = s.normal_at(&Point {
        x: 0.0,
        y: 1.70711,
        z: -0.70711,
    });
    assert_eq!(
        n,
        Vector {
            x: 0.0,
            y: 0.70711,
            z: -0.70711
        }
    );
}
#[test]
fn computing_the_normal_on_a_transformed_sphere() {
    let mut s = Sphere::unit();
    const M: Matrix<4, 4> = Matrix::identity()
        .then(rotation_z(PI / 5.0))
        .then(scaling(1.0, 0.5, 1.0));
    s.set_transform(&M);
    let n = s.normal_at(&Point {
        x: 0.0,
        y: 2.0_f32.sqrt() / 2.0,
        z: -2.0_f32.sqrt() / 2.0,
    });
    assert_eq!(
        n,
        Vector {
            x: 0.0,
            y: 0.97014,
            z: -0.24254
        }
    );
}
#[test]
fn a_sphere_has_a_default_material() {
    let s = Sphere::unit();
    let m = s.material;
    assert_eq!(m, Material::default());
}
#[test]
fn a_sphere_may_be_assigned_a_material() {
    let mut s = Sphere::unit();
    let mut m = Material::default();
    m.set_ambient(1.0);
    s.set_material(&m);
    assert_eq!(s.material, m);
}
