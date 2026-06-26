use crate::intersections::*;
use crate::rays::*;
use crate::shapes::{Primitive, ShapeKind};
use crate::tuples::*;

// The Möller–Trumbore algorithm, shared by both the flat and the smooth
// triangle (their geometry and edge vectors are identical). It both tests for a
// hit and locates it in the triangle's barycentric coordinates (u, v); a ray
// misses unless it lands inside all three edges (u >= 0, v >= 0, u + v <= 1). A
// smooth triangle keeps the (u, v) on the surviving hit so its normal can be
// blended; a flat triangle ignores them.
pub fn triangle_intersect(prim: &Primitive, ray: &Ray, object_id: usize, xs: &mut Intersections) {
    let dir_cross_e2 = ray.direction.cross(prim.e2);
    let det = prim.e1.dot(dir_cross_e2);
    // A determinant near zero means the ray is parallel to the triangle.
    if det.abs() < EPSILON {
        return;
    }

    let f = 1.0 / det;
    let p1_to_origin = ray.origin - prim.p1;
    let u = f * p1_to_origin.dot(dir_cross_e2);
    if u < 0.0 || u > 1.0 {
        return;
    }

    let origin_cross_e1 = p1_to_origin.cross(prim.e1);
    let v = f * ray.direction.dot(origin_cross_e1);
    if v < 0.0 || (u + v) > 1.0 {
        return;
    }

    let t = f * prim.e2.dot(origin_cross_e1);
    match prim.kind {
        ShapeKind::SmoothTriangle => {
            xs.push(Intersection::with_uv(t, object_id, u, v));
        }
        _ => xs.push(Intersection::new(t, object_id)),
    }
}

// Flat surface: the same precomputed normal everywhere, ignoring the point.
pub fn triangle_normal_at(prim: &Primitive) -> Vector {
    prim.normal
}

// Blend the three vertex normals by the hit's barycentric weights. The result
// is normalized when it is lifted into world space by `World::normal_to_world`.
// Without a hit there is no u/v (both zero), so it falls back to the first
// vertex normal.
pub fn smooth_triangle_local_normal_at_uv(prim: &Primitive, u: Number, v: Number) -> Vector {
    prim.n2 * u + prim.n3 * v + prim.n1 * (1.0 - u - v)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The three corners used by most of the book's triangle tests.
    fn example_triangle() -> Primitive {
        Primitive::triangle(
            Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Point {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        )
    }

    #[test]
    fn constructing_a_triangle() {
        let t = example_triangle();
        assert_eq!(
            t.e1,
            Vector {
                x: -1.0,
                y: -1.0,
                z: 0.0
            }
        );
        assert_eq!(
            t.e2,
            Vector {
                x: 1.0,
                y: -1.0,
                z: 0.0
            }
        );
        assert_eq!(
            t.normal,
            Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
    }

    #[test]
    fn finding_the_normal_on_a_triangle() {
        let t = example_triangle();
        let n1 = triangle_normal_at(&t);
        let n2 = triangle_normal_at(&t);
        let n3 = triangle_normal_at(&t);
        assert_eq!(n1, t.normal);
        assert_eq!(n2, t.normal);
        assert_eq!(n3, t.normal);
    }

    #[test]
    fn intersecting_a_ray_parallel_to_the_triangle() {
        let t = example_triangle();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: -1.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let mut xs = Intersections::empty();
        triangle_intersect(&t, &r, 0, &mut xs);
        assert_eq!(xs.count(), 0);
    }

    #[test]
    fn a_ray_misses_the_p1_p3_edge() {
        let t = example_triangle();
        let r = Ray {
            origin: Point {
                x: 1.0,
                y: 1.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let mut xs = Intersections::empty();
        triangle_intersect(&t, &r, 0, &mut xs);
        assert_eq!(xs.count(), 0);
    }

    #[test]
    fn a_ray_misses_the_p1_p2_edge() {
        let t = example_triangle();
        let r = Ray {
            origin: Point {
                x: -1.0,
                y: 1.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let mut xs = Intersections::empty();
        triangle_intersect(&t, &r, 0, &mut xs);
        assert_eq!(xs.count(), 0);
    }

    #[test]
    fn a_ray_misses_the_p2_p3_edge() {
        let t = example_triangle();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: -1.0,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let mut xs = Intersections::empty();
        triangle_intersect(&t, &r, 0, &mut xs);
        assert_eq!(xs.count(), 0);
    }

    #[test]
    fn a_ray_strikes_a_triangle() {
        let t = example_triangle();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.5,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let mut xs = Intersections::empty();
        triangle_intersect(&t, &r, 0, &mut xs);
        assert_eq!(xs.count(), 1);
        assert_almost_eq!(xs[0].t, 2.0);
    }

    // The book's standard smooth triangle: same corners as the flat one, with a
    // normal pointing "outward" at each vertex.
    fn example_smooth_triangle() -> Primitive {
        Primitive::smooth_triangle(
            Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Point {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            Point {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Vector {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        )
    }

    #[test]
    fn constructing_a_smooth_triangle() {
        let t = example_smooth_triangle();
        assert_eq!(t.p1, Point { x: 0.0, y: 1.0, z: 0.0 });
        assert_eq!(t.p2, Point { x: -1.0, y: 0.0, z: 0.0 });
        assert_eq!(t.p3, Point { x: 1.0, y: 0.0, z: 0.0 });
        assert_eq!(t.n1, Vector { x: 0.0, y: 1.0, z: 0.0 });
        assert_eq!(t.n2, Vector { x: -1.0, y: 0.0, z: 0.0 });
        assert_eq!(t.n3, Vector { x: 1.0, y: 0.0, z: 0.0 });
    }

    #[test]
    fn an_intersection_with_a_smooth_triangle_stores_u_v() {
        let t = example_smooth_triangle();
        let r = Ray {
            origin: Point {
                x: -0.2,
                y: 0.3,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let mut xs = Intersections::empty();
        triangle_intersect(&t, &r, 0, &mut xs);
        assert_almost_eq!(xs[0].u, 0.45);
        assert_almost_eq!(xs[0].v, 0.25);
    }

    #[test]
    fn a_smooth_triangle_uses_u_v_to_interpolate_the_normal() {
        use crate::worlds::World;
        let mut w = World::new();
        let t = example_smooth_triangle();
        w.objects.push(t);
        // Resolve the normal through the world, which interpolates from u/v and
        // normalizes the result.
        let n = w.normal_at_uv(
            0,
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            0.45,
            0.25,
        );
        assert_almost_eq!(n.x, -0.5547);
        assert_almost_eq!(n.y, 0.83205);
        assert_almost_eq!(n.z, 0.0);
    }

    #[test]
    fn preparing_the_normal_on_a_smooth_triangle() {
        use crate::worlds::World;
        let mut w = World::new();
        let t = example_smooth_triangle();
        w.objects.push(t);
        let i = Intersection::with_uv(1.0, 0, 0.45, 0.25);
        let r = Ray {
            origin: Point {
                x: -0.2,
                y: 0.3,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = Intersections::new(vec![i]);
        let comps = i.prepare_computations(&r, &w, &xs);
        assert_almost_eq!(comps.normalv.x, -0.5547);
        assert_almost_eq!(comps.normalv.y, 0.83205);
        assert_almost_eq!(comps.normalv.z, 0.0);
    }
}
