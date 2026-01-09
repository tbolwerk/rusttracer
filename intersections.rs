use std::ops::Index;

use crate::intersections;
use crate::rays::*;
use crate::shapes::*;
use crate::tuples::*;
use crate::worlds::World;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Intersection {
    pub t: f32,
    pub object_id: usize,
}
pub struct Computations {
    pub t: f32,
    pub object_id: usize,
    pub point: Point,
    pub eyev: Vector,
    pub normalv: Vector,
    pub inside: bool,
    pub over_point: Point,
    pub reflectv: Vector,
    pub n1: f32,
    pub n2: f32,
    pub under_point: Point,
}

impl Computations {
    pub fn schlick(&self) -> f32 {
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
        let mut containers: Vec<usize> = vec![];
        for i in xs.intersections.iter() {
            if i == self {
                match containers.last() {
                    None => (),
                    Some(object_id) => {
                        n1 = world.objects[*object_id].get_material().refractive_index;
                    }
                }
            }
            if let Some(pos) = containers.iter().position(|&x| x == i.object_id) {
                containers.remove(pos);
            } else {
                containers.push(i.object_id);
            }
            if i == self {
                match containers.last() {
                    None => (),
                    Some(object_id) => {
                        n2 = world.objects[*object_id].get_material().refractive_index;
                    }
                }
            }
        }
        let point = ray.position(self.t);
        let mut inside = false;
        let object = &world.objects[self.object_id];
        let mut normalv = object.normal_at(&point);
        let eyev = -ray.direction;
        if normalv.dot(eyev) < 0.0 {
            inside = true;
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
            under_point,
        }
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

#[derive(Debug, PartialEq)]
pub struct Intersections {
    pub intersections: Vec<Intersection>,
}

impl Intersections {
    pub fn new(xs: Vec<Intersection>) -> Self {
        let mut intersections = xs;
        intersections.sort();
        Self { intersections }
    }

    pub fn hit(&self) -> Option<&Intersection> {
        let mut result = None;
        for intersection in self.intersections.iter() {
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
    pub fn extend(&mut self, mut other: Intersections) -> () {
        self.intersections.append(&mut other.intersections);
        self.intersections.sort();
    }
    pub fn count(&self) -> usize {
        self.intersections.len()
    }
}
impl Index<usize> for Intersections {
    type Output = Intersection;
    fn index(&self, index: usize) -> &Self::Output {
        &self.intersections[index]
    }
}
impl Intersection {
    pub const fn new(t: f32, object_id: usize) -> Self {
        Self { t, object_id }
    }
}
mod tests {
    use crate::{
        materials::Material,
        transformations::{scaling, translation},
    };

    use super::*;

    #[test]
    fn an_intersection_encapsulates_t_and_object() {
        const S: Shape = Shape::sphere();
        let i = Intersection::new(3.5, 0);
        assert_eq!(i.t, 3.5);
        assert_eq!(i.object_id, 0);
    }
    #[test]
    fn aggregating_intersections() {
        const S: Shape = Shape::sphere();
        let i1 = Intersection::new(1.0, 0);
        let i2 = Intersection::new(2.0, 1);
        let xs = Intersections::new(vec![i1, i2]);
        assert_eq!(xs[0].t, 1.0);
        assert_eq!(xs[1].t, 2.0);
    }
    #[test]
    fn the_hit_when_all_intersections_have_positive_t() {
        const S: Shape = Shape::sphere();
        let i1 = Intersection::new(1.0, 0);
        let i2 = Intersection::new(2.0, 1);
        let xs = Intersections::new(vec![i2, i1]);
        let i = xs.hit();
        assert_eq!(i.unwrap(), &i1);
    }
    #[test]
    fn the_hit_when_some_intersections_have_negative_t() {
        const S: Shape = Shape::sphere();
        let i1 = Intersection::new(-1.0, 0);
        let i2 = Intersection::new(1.0, 1);
        let xs = Intersections::new(vec![i2, i1]);
        let i = xs.hit();
        assert_eq!(i.unwrap(), &i2);
    }
    #[test]
    fn the_hit_when_all_intersections_have_negative_t() {
        const S: Shape = Shape::sphere();
        let i1 = Intersection::new(-2.0, 0);
        let i2 = Intersection::new(-1.0, 1);
        let xs = Intersections::new(vec![i2, i1]);
        let i = xs.hit();
        assert_eq!(i, None);
    }
    #[test]
    fn the_hit_is_always_the_lowest_nonnegative_intersection() {
        const S: Shape = Shape::sphere();
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
        let shape = Shape::sphere();
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
        let shape = Shape::sphere();
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
        let shape = Shape::sphere();
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
        let a = Shape::with(
            Shape::glass_sphere,
            scaling(2.0, 2.0, 2.0),
            material.clone(),
        );
        material.set_refractive_index(2.0);
        let b = Shape::with(
            Shape::glass_sphere,
            translation(0.0, 0.0, -0.25),
            material.clone(),
        );
        material.set_refractive_index(2.5);
        let c = Shape::with(Shape::glass_sphere, translation(0.0, 0.0, 0.25), material);
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
        let shape = Shape::with(
            Shape::glass_sphere,
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
        let shape = Shape::glass_sphere();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: 2.0_f32.sqrt() / 2.0,
            },
            direction: Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        };
        let xs = Intersections::new(vec![
            Intersection::new(-2.0_f32.sqrt() / 2.0, 0),
            Intersection::new(2.0_f32.sqrt() / 2.0, 0),
        ]);
        let mut w = World::default();
        w.objects = vec![shape];
        let comps = xs[1].prepare_computations(&r, &w, &xs);
        let reflectance = comps.schlick();
        assert_eq!(reflectance, 1.0);
    }
    #[test]
    fn the_schlick_approximation_with_a_perpendicular_viewing_angle() {
        let shape = Shape::glass_sphere();
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
        // Round two digits precision to keep it aligned with the book.
        let reflectance = (comps.schlick() * 100.0).round() / 100.0;
        assert_eq!(reflectance, 0.04);
    }
    #[test]
    fn the_schlick_approximation_with_small_angle_and_n2_gt_n1() {
        let shape = Shape::glass_sphere();
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
        // Round five digits precision to keep it aligned with the book.
        let reflectance = (comps.schlick() * 100000.0).round() / 100000.0;
        assert_eq!(reflectance, 0.48873);
    }
}
