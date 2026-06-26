use crate::intersections::*;
use crate::rays::*;
use crate::tuples::*;
#[cfg(test)]
use crate::matrices::*;
#[cfg(test)]
use crate::shapes::*;
#[cfg(test)]
use crate::materials::*;
#[cfg(test)]
use crate::transformations::*;

// The unit sphere is centered at the origin with radius 1; all other spheres are
// this one under a transform, so the math below bakes both constants in.
pub fn sphere_intersect(ray: &Ray, object_id: usize) -> Intersections {
    let origin = Point {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let sphere_to_ray = ray.origin - origin;

    let a = ray.direction.dot(ray.direction);
    let b = 2.0 * ray.direction.dot(sphere_to_ray);
    let c = sphere_to_ray.dot(sphere_to_ray) - 1.0;

    let discriminant = b.powi(2) - 4.0 * a * c;

    if discriminant < 0.0 {
        return Intersections::new(vec![]);
    }

    let t1 = (-b - discriminant.sqrt()) / (2.0 * a);
    let t2 = (-b + discriminant.sqrt()) / (2.0 * a);

    let i1 = Intersection::new(t1, object_id);
    let i2 = Intersection::new(t2, object_id);
    Intersections::new(vec![i1, i2])
}

// The unit sphere's normal at a surface point is simply the vector from the
// center (the origin) to that point.
pub fn sphere_normal_at(point: &Point) -> Vector {
    Vector {
        x: point.x(),
        y: point.y(),
        z: point.z(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let xs = sphere_intersect(&R, 0);
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
        let xs = sphere_intersect(&R, 0);
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
        let xs = sphere_intersect(&R, 0);
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
        let xs = sphere_intersect(&R, 0);
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
        let xs = sphere_intersect(&R, 0);
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
        let xs = sphere_intersect(&R, 0);
        Intersections::new(vec![xs[0], xs[1]]);
        assert_eq!(xs[0].object_id, 0);
    }
    #[test]
    fn a_spheres_default_transformation() {
        let s = Primitive::sphere();
        assert_eq!(s.get_transform(), Matrix::identity());
    }
    #[test]
    fn changing_a_spheres_transformation() {
        let mut s = Primitive::sphere();
        const T: Matrix<4, 4> = translation(2.0, 3.0, 4.0);
        s.set_transform(T);
        assert_eq!(s.get_transform(), T);
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
        let mut s = Primitive::sphere();
        s.set_transform(scaling(2.0, 2.0, 2.0));
        let xs = s.intersect(&R, 0);
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
        let mut s = Primitive::sphere();
        s.set_transform(translation(5.0, 0.0, 0.0));
        let xs = s.intersect(&R, 0);
        assert_eq!(xs.count(), 0);
    }
    #[test]
    fn the_normal_on_a_sphere_at_a_point_on_the_x_axis() {
        let n = sphere_normal_at(&Point {
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
        let n = sphere_normal_at(&Point {
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
        let n = sphere_normal_at(&Point {
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
        let n = sphere_normal_at(&Point {
            x: sqrt(3.0) / 3.0,
            y: sqrt(3.0) / 3.0,
            z: sqrt(3.0) / 3.0,
        });
        assert_eq!(
            n,
            Vector {
                x: sqrt(3.0) / 3.0,
                y: sqrt(3.0) / 3.0,
                z: sqrt(3.0) / 3.0,
            }
        );
    }
    #[test]
    fn the_normal_is_a_normalized_vector() {
        let n = sphere_normal_at(&Point {
            x: sqrt(3.0) / 3.0,
            y: sqrt(3.0) / 3.0,
            z: sqrt(3.0) / 3.0,
        });
        assert_eq!(n, n.normalize());
    }
    #[test]
    fn computing_the_normal_on_a_translated_sphere() {
        let mut s = Primitive::sphere();
        s.set_transform(translation(0.0, 1.0, 0.0));
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
        let mut s = Primitive::sphere();
        const M: Matrix<4, 4> = Matrix::identity()
            .then(rotation_z(PI / 5.0))
            .then(scaling(1.0, 0.5, 1.0));
        s.set_transform(M);
        let n = s.normal_at(&Point {
            x: 0.0,
            y: sqrt(2.0) / 2.0,
            z: -sqrt(2.0) / 2.0,
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
        let s = Primitive::sphere();
        let m = s.get_material();
        assert_eq!(m, Material::default());
    }
    #[test]
    fn a_sphere_may_be_assigned_a_material() {
        let mut s = Primitive::sphere();
        let mut m = Material::default();
        m.set_ambient(1.0);
        s.set_material(m.clone());
        assert_eq!(s.get_material(), m);
    }
    #[test]
    fn a_helper_for_producing_a_sphere_with_glassy_material() {
        let s = Primitive::glass_sphere();
        assert_eq!(s.get_transform(), Matrix::identity());
        assert_eq!(s.get_material().transparency, 1.0);
        assert_eq!(s.get_material().refractive_index, 1.5);
    }
}
