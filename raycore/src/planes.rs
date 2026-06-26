use crate::intersections::*;
use crate::rays::*;
use crate::tuples::*;
#[cfg(test)]
use crate::shapes::*;

// The plane lies in the xz axis (y = 0). A ray hits it once, unless it runs
// parallel (its y-direction is ~0).
pub fn plane_intersect(ray: &Ray, object_id: usize) -> Intersections {
    if ray.direction.y().abs() < EPSILON {
        return Intersections::new(vec![]);
    }
    Intersections::new(vec![Intersection::new(
        -ray.origin.y / ray.direction.y,
        object_id,
    )])
}

// A plane's normal points straight up everywhere; the point is irrelevant.
pub fn plane_normal_at(_: &Point) -> Vector {
    Vector {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_normal_of_a_plane_is_constant_everywhere() {
        let n1 = plane_normal_at(&Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
        let n2 = plane_normal_at(&Point {
            x: 10.0,
            y: 0.0,
            z: -10.0,
        });
        let n3 = plane_normal_at(&Point {
            x: -5.0,
            y: 0.0,
            z: 150.0,
        });
        assert_eq!(
            n1,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
        assert_eq!(
            n2,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
        assert_eq!(
            n3,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn intersect_with_a_ray_parallel_to_the_plane() {
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = plane_intersect(&r, 0);
        assert_eq!(xs.count(), 0);
    }
    #[test]
    fn intersect_with_a_coplanar_ray() {
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
        let xs = plane_intersect(&r, 0);
        assert_eq!(xs.count(), 0);
    }
    #[test]
    fn a_plane_intersecting_a_plane_from_above() {
        let p = Primitive::plane();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        };
        let xs = p.intersect(&r, 0);
        assert_eq!(xs.count(), 1);
        assert_eq!(xs[0].t, 1.0);
        assert_eq!(xs[0].object_id, 0);
    }
}
