use crate::intersections::*;
use crate::rays::*;
use crate::tuples::*;

// The axis-aligned unit cube spans [-1, 1] on every axis. The slab method tests
// the ray against each pair of parallel faces and keeps the overlapping range.
pub fn cube_intersect(ray: &Ray, object_id: usize) -> Intersections {
    fn check_axis(origin: Number, direction: Number) -> (Number, Number) {
        let tmin_numerator = -1 as Number - origin;
        let tmax_numerator = 1 as Number - origin;

        let tmin;
        let tmax;

        if direction.abs() >= EPSILON {
            tmin = tmin_numerator / direction;
            tmax = tmax_numerator / direction;
        } else {
            tmin = tmin_numerator * Number::MAX;
            tmax = tmax_numerator * Number::MAX;
        }

        if tmin > tmax {
            return (tmax, tmin);
        }
        (tmin, tmax)
    }

    let (xtmin, xtmax) = check_axis(ray.origin.x, ray.direction.x);
    let (ytmin, ytmax) = check_axis(ray.origin.y, ray.direction.y);
    let (ztmin, ztmax) = check_axis(ray.origin.z, ray.direction.z);

    let tmin = xtmin.max(ytmin).max(ztmin);
    let tmax = xtmax.min(ytmax).min(ztmax);

    if tmin > tmax {
        return Intersections::new(vec![]);
    }

    Intersections::new(vec![
        Intersection::new(tmin, object_id),
        Intersection::new(tmax, object_id),
    ])
}

// The cube's normal points along whichever axis the point's coordinate is
// largest in magnitude, i.e. the face it lies on.
pub fn cube_normal_at(point: &Point) -> Vector {
    let x = point.x().abs();
    let y = point.y().abs();
    let z = point.z().abs();
    let maxc = x.max(y).max(z);

    if maxc == x {
        return Vector {
            x: point.x(),
            y: 0.0,
            z: 0.0,
        };
    }
    if maxc == y {
        return Vector {
            x: 0.0,
            y: point.y(),
            z: 0.0,
        };
    }
    Vector {
        x: 0.0,
        y: 0.0,
        z: point.z(),
    }
}

#[test]
fn a_ray_intersects_a_cube() {
    struct Example {
        name: &'static str,
        origin: Point,
        direction: Vector,
        t1: Number,
        t2: Number,
    }
    let examples = [
        Example {
            name: "+x",
            origin: Point {
                x: 5.0,
                y: 0.5,
                z: 0.0,
            },
            direction: Vector {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            t1: 4.0,
            t2: 6.0,
        },
        Example {
            name: "-x",
            origin: Point {
                x: -5.0,
                y: 0.5,
                z: 0.0,
            },
            direction: Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            t1: 4.0,
            t2: 6.0,
        },
        Example {
            name: "+y",
            origin: Point {
                x: 0.5,
                y: 5.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            t1: 4.0,
            t2: 6.0,
        },
        Example {
            name: "-y",
            origin: Point {
                x: 0.5,
                y: -5.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            t1: 4.0,
            t2: 6.0,
        },
        Example {
            name: "+z",
            origin: Point {
                x: 0.5,
                y: 0.0,
                z: 5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            t1: 4.0,
            t2: 6.0,
        },
        Example {
            name: "-z",
            origin: Point {
                x: 0.5,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            t1: 4.0,
            t2: 6.0,
        },
        Example {
            name: "inside",
            origin: Point {
                x: 0.0,
                y: 0.5,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            t1: -1.0,
            t2: 1.0,
        },
    ];
    for Example {
        name,
        origin,
        direction,
        t1,
        t2,
    } in examples
    {
        let r = Ray { origin, direction };
        let xs = cube_intersect(&r, 2);
        println!("Example {name}");
        assert_eq!(xs.count(), 2);
        assert_eq!(xs[0].t, t1);
        assert_eq!(xs[1].t, t2);
    }
}

#[test]
fn a_ray_misses_a_cube() {
    struct Example {
        origin: Point,
        direction: Vector,
    }
    let examples = [
        Example {
            origin: Point {
                x: -2.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.2673,
                y: 0.5345,
                z: 0.8018,
            },
        },
        Example {
            origin: Point {
                x: 0.0,
                y: -2.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.8018,
                y: 0.2673,
                z: 0.5345,
            },
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.5345,
                y: 0.8018,
                z: 0.2673,
            },
        },
        Example {
            origin: Point {
                x: 2.0,
                y: 0.0,
                z: 2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 2.0,
                z: 2.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        Example {
            origin: Point {
                x: 2.0,
                y: 2.0,
                z: 0.0,
            },
            direction: Vector {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
        },
    ];
    for Example { origin, direction } in examples {
        let ray = Ray { origin, direction };
        let xs = cube_intersect(&ray, 0);
        assert_eq!(xs.count(), 0);
    }
}

#[test]
fn the_normal_on_the_surface_of_a_cube() {
    struct Example {
        point: Point,
        normal: Vector,
    }
    let examples = [
        Example {
            point: Point {
                x: 1.0,
                y: 0.5,
                z: -0.8,
            },
            normal: Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: -1.0,
                y: -0.2,
                z: 0.9,
            },
            normal: Vector {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: -0.4,
                y: 1.0,
                z: -0.1,
            },
            normal: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.3,
                y: -1.0,
                z: -0.7,
            },
            normal: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: -0.6,
                y: 0.3,
                z: 1.0,
            },
            normal: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        },
        Example {
            point: Point {
                x: 0.4,
                y: 0.4,
                z: -1.0,
            },
            normal: Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        },
        Example {
            point: Point {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            normal: Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: -1.0,
                y: -1.0,
                z: -1.0,
            },
            normal: Vector {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
        },
    ];
    for Example { point, normal } in examples {
        let expected_normal = cube_normal_at(&point);
        assert_eq!(normal, expected_normal);
    }
}
