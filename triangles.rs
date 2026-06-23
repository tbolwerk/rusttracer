use crate::intersections::*;
use crate::materials::*;
use crate::rays::*;
use crate::shapes::*;
use crate::tuples::*;

// A flat triangle defined by its three corners. Following the book, the two
// edge vectors and the surface normal are precomputed once at construction:
// they never change (the triangle is rigid in object space), and the
// Möller–Trumbore intersection test below reuses them on every ray.
#[derive(Debug, PartialEq, Clone)]
pub struct Triangle {
    pub transform: TransformData,
    material: Material,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
    pub e1: Vector,
    pub e2: Vector,
    pub normal: Vector,
}

impl Triangle {
    pub fn new(p1: Point, p2: Point, p3: Point) -> Self {
        let e1 = p2 - p1;
        let e2 = p3 - p1;
        // The triangle is flat, so a single normal serves every point on it.
        let normal = e2.cross(e1).normalize();
        Triangle {
            transform: TransformData::default(),
            material: Material::default(),
            p1,
            p2,
            p3,
            e1,
            e2,
            normal,
        }
    }
}

impl HasMaterial for Triangle {
    fn set_material(&mut self, material: Material) -> () {
        self.material = material;
    }
    fn get_material(&self) -> Material {
        self.material.clone()
    }
}

impl Intersects for Triangle {
    // The Möller–Trumbore algorithm. It both tests for a hit and locates it in
    // the triangle's barycentric coordinates (u, v); a ray misses unless it
    // lands inside all three edges (u >= 0, v >= 0, u + v <= 1).
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        let dir_cross_e2 = ray.direction.cross(self.e2);
        let det = self.e1.dot(dir_cross_e2);
        // A determinant near zero means the ray is parallel to the triangle.
        if det.abs() < EPSILON {
            return Intersections::new(vec![]);
        }

        let f = 1.0 / det;
        let p1_to_origin = ray.origin - self.p1;
        let u = f * p1_to_origin.dot(dir_cross_e2);
        if u < 0.0 || u > 1.0 {
            return Intersections::new(vec![]);
        }

        let origin_cross_e1 = p1_to_origin.cross(self.e1);
        let v = f * ray.direction.dot(origin_cross_e1);
        if v < 0.0 || (u + v) > 1.0 {
            return Intersections::new(vec![]);
        }

        let t = f * self.e2.dot(origin_cross_e1);
        Intersections::new(vec![Intersection::new(t, object_id)])
    }
    // Flat surface: the same normal everywhere, ignoring the point.
    fn local_normal_at(&self, _point: &Point) -> Vector {
        self.normal
    }
}

// A triangle that fakes a curved surface by interpolating a normal across its
// face from a normal stored at each vertex (Phong/"smooth" shading). The hit
// test is identical to the flat triangle's, but it records the barycentric
// (u, v) of the hit so the normal can be blended at shading time.
#[derive(Debug, PartialEq, Clone)]
pub struct SmoothTriangle {
    pub transform: TransformData,
    material: Material,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
    pub n1: Vector,
    pub n2: Vector,
    pub n3: Vector,
    pub e1: Vector,
    pub e2: Vector,
}

impl SmoothTriangle {
    pub fn new(p1: Point, p2: Point, p3: Point, n1: Vector, n2: Vector, n3: Vector) -> Self {
        SmoothTriangle {
            transform: TransformData::default(),
            material: Material::default(),
            p1,
            p2,
            p3,
            n1,
            n2,
            n3,
            e1: p2 - p1,
            e2: p3 - p1,
        }
    }
}

impl HasMaterial for SmoothTriangle {
    fn set_material(&mut self, material: Material) -> () {
        self.material = material;
    }
    fn get_material(&self) -> Material {
        self.material.clone()
    }
}

impl Intersects for SmoothTriangle {
    // Identical Möller–Trumbore test to the flat triangle, except the surviving
    // hit keeps its (u, v) so `local_normal_at_uv` can interpolate the normal.
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        let dir_cross_e2 = ray.direction.cross(self.e2);
        let det = self.e1.dot(dir_cross_e2);
        if det.abs() < EPSILON {
            return Intersections::new(vec![]);
        }

        let f = 1.0 / det;
        let p1_to_origin = ray.origin - self.p1;
        let u = f * p1_to_origin.dot(dir_cross_e2);
        if u < 0.0 || u > 1.0 {
            return Intersections::new(vec![]);
        }

        let origin_cross_e1 = p1_to_origin.cross(self.e1);
        let v = f * ray.direction.dot(origin_cross_e1);
        if v < 0.0 || (u + v) > 1.0 {
            return Intersections::new(vec![]);
        }

        let t = f * self.e2.dot(origin_cross_e1);
        Intersections::new(vec![Intersection::with_uv(t, object_id, u, v)])
    }
    // Blend the three vertex normals by the hit's barycentric weights. The result
    // is normalized when it is lifted into world space by `World::normal_to_world`.
    fn local_normal_at_uv(&self, _point: &Point, u: Number, v: Number) -> Vector {
        self.n2 * u + self.n3 * v + self.n1 * (1.0 - u - v)
    }
    // Without a hit there is no u/v, so fall back to the first vertex normal.
    fn local_normal_at(&self, point: &Point) -> Vector {
        self.local_normal_at_uv(point, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The three corners used by most of the book's triangle tests.
    fn example_triangle() -> Triangle {
        Triangle::new(
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
        let n1 = t.local_normal_at(&Point {
            x: 0.0,
            y: 0.5,
            z: 0.0,
        });
        let n2 = t.local_normal_at(&Point {
            x: -0.5,
            y: 0.75,
            z: 0.0,
        });
        let n3 = t.local_normal_at(&Point {
            x: 0.5,
            y: 0.25,
            z: 0.0,
        });
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
        let xs = t.local_intersect(&r, 0);
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
        let xs = t.local_intersect(&r, 0);
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
        let xs = t.local_intersect(&r, 0);
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
        let xs = t.local_intersect(&r, 0);
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
        let xs = t.local_intersect(&r, 0);
        assert_eq!(xs.count(), 1);
        assert_almost_eq!(xs[0].t, 2.0);
    }

    // The book's standard smooth triangle: same corners as the flat one, with a
    // normal pointing "outward" at each vertex.
    fn example_smooth_triangle() -> SmoothTriangle {
        SmoothTriangle::new(
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
        let xs = t.local_intersect(&r, 0);
        assert_almost_eq!(xs[0].u, 0.45);
        assert_almost_eq!(xs[0].v, 0.25);
    }

    #[test]
    fn a_smooth_triangle_uses_u_v_to_interpolate_the_normal() {
        use crate::worlds::World;
        let mut w = World::new();
        let t = example_smooth_triangle();
        w.objects.push(Shape::SmoothTriangle(t));
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
        w.objects.push(Shape::SmoothTriangle(t));
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
