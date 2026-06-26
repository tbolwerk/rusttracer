use crate::intersections::*;
use crate::rays::*;
use crate::shapes::Primitive;
use crate::tuples::*;

// A (double-)cone truncated to a y-range and optionally end-capped, mirroring
// the cylinder. At each y the cone's radius equals |y|, which is why the cap
// radius test uses `bound.abs()`.
fn intersect_caps(prim: &Primitive, ray: &Ray, object_id: usize, xs: &mut Intersections) {
    if prim.closed == 0 || almost_eq(ray.direction.y(), 0.0) {
        return;
    }

    fn check_caps(ray: &Ray, t: Number, radius: Number) -> bool {
        let x = ray.origin.x() + t * ray.direction.x();
        let z = ray.origin.z() + t * ray.direction.z();

        (x.powf(2.0) + z.powf(2.0)) <= radius.powf(2.0) + EPSILON
    }

    // Index loop, not `for bound in [..]`: rust-gpu can't lower array IntoIter.
    let bounds = [prim.minimum, prim.maximum];
    let mut bi = 0;
    while bi < 2 {
        let bound = bounds[bi];
        let t = (bound - ray.origin.y()) / ray.direction.y();
        if check_caps(ray, t, bound.abs()) {
            xs.push(Intersection::new(t, object_id));
        }
        bi += 1;
    }
}

pub fn cone_intersect(prim: &Primitive, ray: &Ray, object_id: usize, xs: &mut Intersections) {
    let a = ray.direction.x().powi(2) - ray.direction.y().powi(2) + ray.direction.z().powi(2);

    let b = 2.0 * ray.origin.x() * ray.direction.x() - 2.0 * ray.origin.y() * ray.direction.y()
        + 2.0 * ray.origin.z() * ray.direction.z();

    let c = ray.origin.x().powi(2) - ray.origin.y().powi(2) + ray.origin.z().powi(2);

    if almost_eq(a, 0.0) {
        // Ray is parallel to one of the cone's halves. With a == 0 there is a
        // single wall intersection (when b != 0); we still need to test the
        // caps afterwards, so don't return early here.
        if !almost_eq(b, 0.0) {
            let t = -c / (2.0 * b);
            xs.push(Intersection::new(t, object_id));
        }
    } else {
        let disc = b.powi(2) - 4.0 * a * c;

        // A truly missing ray has a clearly negative discriminant. A tangent
        // ray gives disc == 0 in exact math, but f32 rounding can push it
        // slightly negative, so tolerate that and clamp before the sqrt.
        if disc >= -EPSILON {
            let disc = disc.max(0.0);

            let mut t0 = (-b - sqrt(disc)) / (2.0 * a);
            let mut t1 = (-b + sqrt(disc)) / (2.0 * a);

            if t0 > t1 {
                (t0, t1) = (t1, t0);
            }

            let ts = [t0, t1];
            let mut ti = 0;
            while ti < 2 {
                let t = ts[ti];
                let y = ray.origin.y() + t * ray.direction.y();
                if prim.minimum < y && y < prim.maximum {
                    xs.push(Intersection::new(t, object_id));
                }
                ti += 1;
            }
        }
    }

    intersect_caps(prim, ray, object_id, xs);
}

pub fn cone_normal_at(prim: &Primitive, point: &Point) -> Vector {
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
    let mut y = (point.x().powi(2) + point.z().powi(2)).sqrt();
    if point.y() > 0.0 {
        y = -y;
    }
    Vector {
        x: point.x(),
        y,
        z: point.z(),
    }
}

#[test]
fn intersecting_a_cone_with_a_ray() {
    let shape = Primitive::cone();
    struct Example {
        origin: Point,
        direction: Vector,
        t0: Number,
        t1: Number,
    }
    let examples = [
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
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            t0: 8.66025,
            t1: 8.66025,
        },
        Example {
            origin: Point {
                x: 1.0,
                y: 1.0,
                z: -5.0,
            },
            direction: Vector {
                x: -0.5,
                y: -1.0,
                z: 1.0,
            },
            t0: 4.55006,
            t1: 49.44994,
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
        cone_intersect(&shape, &r, 0, &mut xs);
        assert_eq!(xs.count(), 2);
        // Looser tolerance: f32 rounding on these large t-values exceeds EPSILON.
        assert_almost_eq!(xs[0].t, t0, 1e-3);
        assert_almost_eq!(xs[1].t, t1, 1e-3);
    }
}
#[test]
fn intersecting_a_cone_with_a_ray_parallel_to_one_of_its_halves() {
    let shape = Primitive::cone();
    let direction = Vector {
        x: 0.0,
        y: 1.0,
        z: 1.0,
    }
    .normalize();
    let r = Ray {
        origin: Point {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        direction,
    };
    let mut xs = Intersections::empty();
    cone_intersect(&shape, &r, 0, &mut xs);
    assert_eq!(xs.count(), 1);
    assert_almost_eq!(xs[0].t, 0.35355);
}
#[test]
fn computing_the_normal_vector_on_a_cone() {
    let shape = Primitive::cone();
    struct Example {
        point: Point,
        normal: Vector,
    }
    let examples = [
        Example {
            point: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            normal: Vector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
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
                y: -sqrt(2.0),
                z: 1.0,
            },
        },
        Example {
            point: Point {
                x: -1.0,
                y: -1.0,
                z: 0.0,
            },
            normal: Vector {
                x: -1.0,
                y: 1.0,
                z: 0.0,
            },
        },
    ];
    for Example { point, normal } in examples {
        let n = cone_normal_at(&shape, &point);
        assert_almost_eq!(n.x, normal.x);
        assert_almost_eq!(n.y, normal.y);
        assert_almost_eq!(n.z, normal.z);
    }
}

#[test]
fn intersecting_a_cones_end_caps() {
    let mut shape = Primitive::cone();
    shape.minimum = -0.5;
    shape.maximum = 0.5;
    shape.closed = 1;
    struct Example {
        origin: Point,
        direction: Vector,
        count: usize,
    }
    let examples = [
        Example {
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
            count: 0,
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -0.25,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 1.0,
            },
            count: 2,
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -0.25,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            count: 4,
        },
    ];
    for Example {
        origin,
        direction,
        count,
    } in examples
    {
        let dir = direction.normalize();
        let r = Ray {
            origin,
            direction: dir,
        };
        let mut xs = Intersections::empty();
        cone_intersect(&shape, &r, 0, &mut xs);
        assert_eq!(xs.count(), count);
    }
}
