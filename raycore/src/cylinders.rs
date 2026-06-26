use crate::intersections::*;
use crate::rays::*;
use crate::shapes::Primitive;
use crate::tuples::*;

// A cylinder may be truncated to a y-range [minimum, maximum] and optionally
// capped at each end (`closed`). When closed, the caps add up to two more
// intersections where the ray crosses each end disc within the unit radius.
fn intersect_caps(prim: &Primitive, ray: &Ray, object_id: usize, xs: &mut Intersections) {
    if !prim.closed || almost_eq(ray.direction.y(), 0.0) {
        return;
    }

    fn check_caps(ray: &Ray, t: Number) -> bool {
        let x = ray.origin.x() + t * ray.direction.x();
        let z = ray.origin.z() + t * ray.direction.z();

        (x.powf(2.0) + z.powf(2.0)) <= 1.0 + EPSILON
    }

    let bounds = [prim.minimum, prim.maximum];
    for bound in bounds {
        let t = (bound - ray.origin.y()) / ray.direction.y();
        if check_caps(ray, t) {
            xs.push(Intersection::new(t, object_id));
        }
    }
}

pub fn cylinder_intersect(prim: &Primitive, ray: &Ray, object_id: usize, xs: &mut Intersections) {
    let a = ray.direction.x().powi(2) + ray.direction.z().powi(2);

    if almost_eq(a, 0.0) {
        intersect_caps(prim, ray, object_id, xs);

        return;
    }

    let b = 2.0 * ray.origin.x() * ray.direction.x() + 2.0 * ray.origin.z() * ray.direction.z();
    let c = ray.origin.x().powi(2) + ray.origin.z().powi(2) - 1.0;

    let disc = b.powi(2) - 4.0 * a * c;

    if disc < 0.0 {
        return;
    }

    let mut t0 = (-b - sqrt(disc)) / (2.0 * a);
    let mut t1 = (-b + sqrt(disc)) / (2.0 * a);

    if t0 > t1 {
        (t0, t1) = (t1, t0);
    }

    for t in [t0, t1] {
        let y = ray.origin.y() + t * ray.direction.y();
        if prim.minimum < y && y < prim.maximum {
            xs.push(Intersection::new(t, object_id));
        }
    }

    intersect_caps(prim, ray, object_id, xs);
}

pub fn cylinder_normal_at(prim: &Primitive, point: &Point) -> Vector {
    let dist = point.x().powi(2) + point.z().powi(2);

    if dist < 1.0 + EPSILON && point.y() >= prim.maximum - EPSILON {
        return Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
    }
    if dist < 1.0 + EPSILON && point.y() <= prim.minimum + EPSILON {
        return Vector {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        };
    }
    Vector {
        x: point.x(),
        y: 0.0,
        z: point.z(),
    }
}

#[test]
fn a_ray_misses_a_cylinder() {
    let cyl = Primitive::cylinder();
    struct Example {
        origin: Point,
        direction: Vector,
    }
    let examples = [
        Example {
            origin: Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    ];
    for Example { origin, direction } in examples {
        let dir = direction.normalize();
        let r = Ray {
            origin: origin,
            direction: dir,
        };
        let mut xs = Intersections::empty();
        cylinder_intersect(&cyl, &r, 0, &mut xs);
        assert_eq!(xs.count(), 0);
    }
}

#[test]
fn a_ray_strikes_a_cylinder() {
    let cyl = Primitive::cylinder();
    struct Example {
        origin: Point,
        direction: Vector,
        t0: Number,
        t1: Number,
    }
    let examples = [
        Example {
            origin: Point {
                x: 1.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            t0: 5.0,
            t1: 5.0,
        },
        Example {
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
            t0: 4.0,
            t1: 6.0,
        },
        Example {
            origin: Point {
                x: 0.5,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.1,
                y: 1.0,
                z: 1.0,
            },
            t0: 6.80798,
            t1: 7.08872,
        },
    ];
    for Example {
        origin,
        direction,
        t0,
        t1,
    } in examples
    {
        let dir = direction.normalize();
        let r = Ray {
            origin,
            direction: dir,
        };
        let mut xs = Intersections::empty();
        cylinder_intersect(&cyl, &r, 0, &mut xs);
        assert_eq!(xs.count(), 2);
        // Looser tolerance: f32 rounding on these t-values exceeds EPSILON.
        assert_almost_eq!(xs[0].t, t0, 1e-3);
        assert_almost_eq!(xs[1].t, t1, 1e-3);
    }
}
#[test]
fn normal_vector_on_a_cylinder() {
    let cyl = Primitive::cylinder();
    struct Example {
        point: Point,
        normal: Vector,
    }
    let examples = [
        Example {
            point: Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            normal: Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.0,
                y: 5.0,
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
                x: 0.0,
                y: -2.0,
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
                x: -1.0,
                y: 1.0,
                z: 0.0,
            },
            normal: Vector {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
        },
    ];
    for Example { point, normal } in examples {
        let n = cylinder_normal_at(&cyl, &point);
        assert_eq!(n, normal);
    }
}

#[test]
fn the_default_minimum_and_maximum_for_a_cylinder() {
    let cyl = Primitive::cylinder();
    assert_eq!(cyl.minimum, Number::MIN);
    assert_eq!(cyl.maximum, Number::MAX);
}
#[test]
fn intersecting_a_contrained_cylinder() {
    let mut cyl = Primitive::cylinder();
    cyl.minimum = 1.0;
    cyl.maximum = 2.0;
    struct Example {
        point: Point,
        direction: Vector,
        count: usize,
    }
    let examples = [
        Example {
            point: Point {
                x: 0.0,
                y: 1.5,
                z: 0.0,
            },
            direction: Vector {
                x: 0.1,
                y: 1.0,
                z: 0.0,
            },
            count: 0,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 3.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            count: 0,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            count: 0,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 2.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            count: 0,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 1.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            count: 0,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 1.5,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            count: 2,
        },
    ];
    for Example {
        point,
        direction,
        count,
    } in examples
    {
        let dir = direction.normalize();
        let r = Ray {
            origin: point,
            direction: dir,
        };
        let mut xs = Intersections::empty();
        cylinder_intersect(&cyl, &r, 0, &mut xs);
        assert_eq!(xs.count(), count);
    }
}
#[test]
fn the_default_closed_value_for_a_cylinder() {
    let cyl = Primitive::cylinder();
    assert_eq!(cyl.closed, false);
}
#[test]
fn intersecting_the_caps_of_a_closed_cylinder() {
    let mut cyl = Primitive::cylinder();
    cyl.minimum = 1.0;
    cyl.maximum = 2.0;
    cyl.closed = true;
    struct Example {
        point: Point,
        direction: Vector,
        count: usize,
    }
    let examples = [
        Example {
            point: Point {
                x: 0.0,
                y: 3.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            count: 2,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 3.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 2.0,
            },
            count: 2,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 4.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 1.0,
            },
            count: 2,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 0.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            },
            count: 2,
        },
        Example {
            point: Point {
                x: 0.0,
                y: 0.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 2.0,
            },
            count: 2,
        },
        Example {
            point: Point {
                x: 0.0,
                y: -1.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 1.0,
            },
            count: 2,
        },
    ];
    for (
        i,
        Example {
            point,
            direction,
            count,
        },
    ) in examples.iter().enumerate()
    {
        let dir = direction.normalize();
        let r = Ray {
            origin: *point,
            direction: dir,
        };
        let mut xs = Intersections::empty();
        cylinder_intersect(&cyl, &r, 0, &mut xs);
        println!("Example no. {i}");
        assert_eq!(xs.count(), *count);
    }
}
#[test]
fn the_normal_vector_on_a_cylinders_end_caps() {
    let mut cyl = Primitive::cylinder();
    cyl.minimum = 1.0;
    cyl.maximum = 2.0;
    cyl.closed = true;
    struct Example {
        point: Point,
        normal: Vector,
    }
    let examples = [
        Example {
            point: Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            normal: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.5,
                y: 1.0,
                z: 0.0,
            },
            normal: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.0,
                y: 1.0,
                z: 0.5,
            },
            normal: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.0,
                y: 2.0,
                z: 0.0,
            },
            normal: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.5,
                y: 2.0,
                z: 0.0,
            },
            normal: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 0.0,
                y: 2.0,
                z: 0.5,
            },
            normal: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        },
    ];
    for Example { point, normal } in examples {
        let n = cylinder_normal_at(&cyl, &point);
        assert_eq!(n, normal);
    }
}
