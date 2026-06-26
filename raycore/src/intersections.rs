use std::ops::Index;

use crate::rays::*;
use crate::shapes::*;
use crate::tuples::*;
use crate::worlds::World;

// Maximum number of intersections tracked per ray. The intersection buffer is a
// fixed-capacity array of this size so the ray path allocates nothing on the
// heap (required for rust-gpu / no_std). Any intersections beyond this cap are
// silently dropped (`push` debug-asserts to surface overflow in debug builds).
pub const MAX_XS: usize = 256;

#[derive(Debug, Copy, Clone, Default)]
pub struct Intersection {
    pub t: Number,
    pub object_id: usize,
    // Barycentric coordinates of the hit on a (smooth) triangle. They are unused
    // by every other shape and left at 0.0; a smooth triangle fills them in via
    // `Intersection::with_uv` so its normal can be interpolated across the face.
    pub u: Number,
    pub v: Number,
}
pub struct Computations {
    pub t: Number,
    pub object_id: usize,
    pub point: Point,
    pub eyev: Vector,
    pub normalv: Vector,
    pub inside: bool,
    pub over_point: Point,
    pub reflectv: Vector,
    pub n1: Number,
    pub n2: Number,
    pub under_point: Point,
}

impl Computations {
    pub fn schlick(&self) -> Number {
        let mut cos = self.eyev.dot(self.normalv);

        if self.n1 > self.n2 {
            let n = self.n1 / self.n2;
            let sin2_t = n.powi(2) * (1.0 - cos.powi(2));
            if sin2_t > 1.0 {
                return 1.0;
            }

            let cos_t = (1.0 - sin2_t).sqrt();

            cos = cos_t;
        }

        let r0 = ((self.n1 - self.n2) / (self.n1 + self.n2)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cos).powi(5)
    }
}
impl Intersection {
    pub fn prepare_computations(
        &self,
        ray: &Ray,
        world: &World,
        xs: &Intersections,
    ) -> Computations {
        let mut n1 = 1.0;
        let mut n2 = 1.0;
        // Fixed-capacity stand-in for the book's `containers` Vec: the set of
        // objects the ray is currently inside, tracked over `containers[0..clen]`.
        let mut containers = [0usize; MAX_XS];
        let mut clen = 0usize;
        for idx in 0..xs.len {
            let i = xs.xs[idx];
            let is_hit = i.t == self.t;
            if is_hit {
                if clen > 0 {
                    let object_id = containers[clen - 1];
                    n1 = world.objects[object_id].get_material().refractive_index;
                }
            }
            // Find `i.object_id` in containers[0..clen]; if present remove it
            // (shift the tail left), otherwise append it.
            let mut pos = None;
            for c in 0..clen {
                if containers[c] == i.object_id {
                    pos = Some(c);
                    break;
                }
            }
            match pos {
                Some(p) => {
                    for c in p..clen - 1 {
                        containers[c] = containers[c + 1];
                    }
                    clen -= 1;
                }
                None => {
                    if clen < MAX_XS {
                        containers[clen] = i.object_id;
                        clen += 1;
                    }
                }
            }
            if is_hit {
                if clen > 0 {
                    let object_id = containers[clen - 1];
                    n2 = world.objects[object_id].get_material().refractive_index;
                }
            }
        }
        let point = ray.position(self.t);
        // Resolve the normal through the world so any enclosing group
        // transforms are applied (world_to_object / normal_to_world). The hit's
        // u/v are passed along so a smooth triangle can interpolate its normal;
        // every other shape ignores them.
        let mut normalv = world.normal_at_uv(self.object_id, point, self.u, self.v);
        let eyev = -ray.direction;
        let inside = normalv.dot(eyev) < 0.0;
        if inside {
            normalv = -normalv;
        }
        let over_point = point + normalv * EPSILON;
        let under_point = point - normalv * EPSILON;
        let reflectv = ray.direction.reflect(normalv);
        Computations {
            t: self.t,
            object_id: self.object_id,
            point: point,
            eyev: eyev,
            normalv: normalv,
            inside: inside,
            over_point: over_point,
            reflectv: reflectv,
            n1: n1,
            n2: n2,
            under_point: under_point,
        }
    }
}
impl PartialEq for Intersection {
    fn eq(&self, other: &Self) -> bool {
        // Objects must match exactly, and t must be within epsilon
        self.object_id == other.object_id && (self.t - other.t).abs() < EPSILON
    }
}
impl Eq for Intersection {}
impl Ord for Intersection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.t < other.t {
            return std::cmp::Ordering::Less;
        } else if self.t > other.t {
            return std::cmp::Ordering::Greater;
        }
        std::cmp::Ordering::Equal
    }
}
impl PartialOrd for Intersection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.t < other.t {
            return Some(std::cmp::Ordering::Less);
        } else if self.t > other.t {
            return Some(std::cmp::Ordering::Greater);
        }
        Some(std::cmp::Ordering::Equal)
    }
}

// A fixed-capacity buffer of intersections. `xs[0..len]` are the live entries;
// the rest are unused padding. Heap-free so the ray path can run under no_std /
// rust-gpu. Clone (a 256-element copy), not Copy, to keep moves cheap by default.
#[derive(Debug, Clone)]
pub struct Intersections {
    pub xs: [Intersection; MAX_XS],
    pub len: usize,
}

impl Intersections {
    pub fn empty() -> Self {
        Self {
            xs: [Intersection::default(); MAX_XS],
            len: 0,
        }
    }
    // Append one intersection. Beyond MAX_XS it is silently dropped; the
    // debug_assert flags the overflow in debug builds.
    pub fn push(&mut self, i: Intersection) {
        if self.len < MAX_XS {
            self.xs[self.len] = i;
            self.len += 1;
        } else {
            debug_assert!(false, "Intersections overflow: MAX_XS ({MAX_XS}) exceeded");
        }
    }
    pub fn count(&self) -> usize {
        self.len
    }
    pub fn hit(&self) -> Option<&Intersection> {
        let mut result = None;
        for idx in 0..self.len {
            let intersection = &self.xs[idx];
            if intersection.t > 0.0 {
                match result {
                    None => result = Some(intersection),
                    Some(intermediate_result) => {
                        if intermediate_result.t > intersection.t {
                            result = Some(intersection);
                        }
                    }
                }
            }
        }
        result
    }
    // Append without sorting. Sorting on every append made a scene-wide intersect
    // do O(objects) sorts of a growing list. Callers that need t-order sort once
    // at the point of use: `intersect_world` before returning, and
    // `filter_intersections` for CSG. `hit()` scans linearly and needs no order.
    pub fn extend(&mut self, other: &Intersections) -> () {
        for idx in 0..other.len {
            self.push(other.xs[idx]);
        }
    }
    // Stable insertion sort of xs[0..len] ascending by `t`. Hand-written (not
    // slice::sort) so it works under no_std later.
    pub fn sort(&mut self) {
        let mut i = 1;
        while i < self.len {
            let key = self.xs[i];
            let mut j = i;
            while j > 0 && self.xs[j - 1].t > key.t {
                self.xs[j] = self.xs[j - 1];
                j -= 1;
            }
            self.xs[j] = key;
            i += 1;
        }
    }
    // Build from a Vec, copying items in and sorting. Test-only: it keeps every
    // existing `Intersections::new(vec![...])` test working verbatim.
    #[cfg(test)]
    pub fn new(v: Vec<Intersection>) -> Self {
        let mut result = Self::empty();
        for i in v {
            result.push(i);
        }
        result.sort();
        result
    }
}
impl Index<usize> for Intersections {
    type Output = Intersection;
    fn index(&self, index: usize) -> &Self::Output {
        &self.xs[index]
    }
}
impl Intersection {
    pub const fn new(t: Number, object_id: usize) -> Self {
        Self {
            t,
            object_id,
            u: 0.0,
            v: 0.0,
        }
    }
    // Used by smooth triangles, which record where on the face the ray landed so
    // the surface normal can be interpolated from the three vertex normals.
    pub const fn with_uv(t: Number, object_id: usize, u: Number, v: Number) -> Self {
        Self { t, object_id, u, v }
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        materials::Material,
        transformations::{scaling, translation},
    };

    use super::*;

    #[test]
    fn an_intersection_encapsulates_t_and_object() {
        let i = Intersection::new(3.5, 0);
        assert_eq!(i.t, 3.5);
        assert_eq!(i.object_id, 0);
    }
    #[test]
    fn an_intersection_can_encapsulate_u_and_v() {
        // A smooth-triangle intersection carries where on the face it landed.
        let i = Intersection::with_uv(3.5, 0, 0.2, 0.4);
        assert_eq!(i.u, 0.2);
        assert_eq!(i.v, 0.4);
    }
    #[test]
    fn aggregating_intersections() {
        let i1 = Intersection::new(1.0, 0);
        let i2 = Intersection::new(2.0, 1);
        let xs = Intersections::new(vec![i1, i2]);
        assert_eq!(xs[0].t, 1.0);
        assert_eq!(xs[1].t, 2.0);
    }
    #[test]
    fn the_hit_when_all_intersections_have_positive_t() {
        let i1 = Intersection::new(1.0, 0);
        let i2 = Intersection::new(2.0, 1);
        let xs = Intersections::new(vec![i2, i1]);
        let i = xs.hit();
        assert_eq!(i.unwrap(), &i1);
    }
    #[test]
    fn the_hit_when_some_intersections_have_negative_t() {
        let i1 = Intersection::new(-1.0, 0);
        let i2 = Intersection::new(1.0, 1);
        let xs = Intersections::new(vec![i2, i1]);
        let i = xs.hit();
        assert_eq!(i.unwrap(), &i2);
    }
    #[test]
    fn the_hit_when_all_intersections_have_negative_t() {
        let i1 = Intersection::new(-2.0, 0);
        let i2 = Intersection::new(-1.0, 1);
        let xs = Intersections::new(vec![i2, i1]);
        let i = xs.hit();
        assert_eq!(i, None);
    }
    #[test]
    fn the_hit_is_always_the_lowest_nonnegative_intersection() {
        let i1 = Intersection::new(5.0, 0);
        let i2 = Intersection::new(7.0, 1);
        let i3 = Intersection::new(-3.0, 2);
        let i4 = Intersection::new(2.0, 3);
        let xs = Intersections::new(vec![i1, i2, i3, i4]);
        let i = xs.hit();
        assert_eq!(i.unwrap(), &i4);
    }
    #[test]
    fn precomputing_the_state_of_an_intersection() {
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
        let mut w = World::new();
        let shape = Primitive::sphere();
        w.objects.append(&mut vec![shape]);
        let i = Intersection::new(4.0, 0);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(comps.t, i.t);
        assert_eq!(comps.object_id, i.object_id);
        assert_eq!(
            comps.point,
            Point {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
        assert_eq!(
            comps.eyev,
            Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
        assert_eq!(
            comps.normalv,
            Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
    }
    #[test]
    fn the_hit_when_an_intersection_occurs_on_the_outside() {
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
        let shape = Primitive::sphere();
        let i = Intersection::new(4.0, 0);
        let mut w = World::new();
        w.objects.append(&mut vec![shape]);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(comps.inside, false);
    }
    #[test]
    fn the_hit_when_an_intersection_occurs_on_the_inside() {
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
        let shape = Primitive::sphere();
        let i = Intersection::new(1.0, 0);
        let mut w = World::new();
        w.objects.append(&mut vec![shape]);
        let comps = i.prepare_computations(&r, &w, &Intersections::new(vec![]));
        assert_eq!(
            comps.point,
            Point {
                x: 0.0,
                y: 0.0,
                z: 1.0
            }
        );
        assert_eq!(
            comps.eyev,
            Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
        assert_eq!(comps.inside, true);
        assert_eq!(
            comps.normalv,
            Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
    }
    #[test]
    fn finding_n1_and_n2_at_various_intersections() {
        let mut material = Material::default();
        material.set_refractive_index(1.5);
        let a = Primitive::with(
            Primitive::glass_sphere,
            scaling(2.0, 2.0, 2.0),
            material.clone(),
        );
        material.set_refractive_index(2.0);
        let b = Primitive::with(
            Primitive::glass_sphere,
            translation(0.0, 0.0, -0.25),
            material.clone(),
        );
        material.set_refractive_index(2.5);
        let c = Primitive::with(Primitive::glass_sphere, translation(0.0, 0.0, 0.25), material);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -4.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        let xs = Intersections::new(vec![
            Intersection::new(2.0, 0),
            Intersection::new(2.75, 1),
            Intersection::new(3.25, 2),
            Intersection::new(4.75, 1),
            Intersection::new(5.25, 2),
            Intersection::new(6.0, 0),
        ]);
        let mut w = World::default();
        let examples = [
            [1.0, 1.5],
            [1.5, 2.0],
            [2.0, 2.5],
            [2.5, 2.5],
            [2.5, 1.5],
            [1.5, 1.0],
        ];
        w.objects = vec![a, b, c];
        for index in 0..xs.count() {
            let comps = xs[index].prepare_computations(&r, &w, &xs);
            assert_eq!(comps.n1, examples[index][0]);
            assert_eq!(comps.n2, examples[index][1]);
        }
    }
    #[test]
    fn the_under_point_is_the_offset_below_the_surface() {
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
        let shape = Primitive::with(
            Primitive::glass_sphere,
            translation(0.0, 0.0, 1.0),
            Material::default(),
        );
        let i = Intersection::new(5.0, 0);
        let xs = Intersections::new(vec![i]);
        let mut w = World::default();
        w.objects = vec![shape];

        let comps = i.prepare_computations(&r, &w, &xs);
        assert_eq!(comps.under_point.z > EPSILON / 2.0, true);
        assert_eq!(comps.point.z < comps.under_point.z, true);
    }
    #[test]
    fn the_schlick_approximation_under_total_internal_reflection() {
        let shape = Primitive::glass_sphere();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: sqrt(2.0) / 2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let xs = Intersections::new(vec![
            Intersection::new(-sqrt(2.0) / 2.0, 0),
            Intersection::new(sqrt(2.0) / 2.0, 0),
        ]);
        let mut w = World::default();
        w.objects = vec![shape];
        let comps = xs[1].prepare_computations(&r, &w, &xs);
        let reflectance = comps.schlick();
        assert_eq!(reflectance, 1.0);
    }
    #[test]
    fn the_schlick_approximation_with_a_perpendicular_viewing_angle() {
        let shape = Primitive::glass_sphere();
        let r = Ray {
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
        };
        let xs = Intersections::new(vec![Intersection::new(-1.0, 0), Intersection::new(1.0, 0)]);
        let mut w = World::default();
        w.objects = vec![shape];
        let comps = xs[1].prepare_computations(&r, &w, &xs);
        let reflectance = comps.schlick();
        assert_almost_eq!(reflectance, 0.04);
    }
    #[test]
    fn the_schlick_approximation_with_small_angle_and_n2_gt_n1() {
        let shape = Primitive::glass_sphere();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.99,
                z: -2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = Intersections::new(vec![Intersection::new(1.8589, 0)]);
        let mut w = World::default();
        w.objects = vec![shape];
        let comps = xs[0].prepare_computations(&r, &w, &xs);
        let reflectance = comps.schlick();
        assert_almost_eq!(reflectance, 0.48873);
    }
}
