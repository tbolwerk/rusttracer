use std::ops::Index;

use crate::intersections;
use crate::rays::*;
use crate::spheres::*;
use crate::tuples::external_tuples::*;
use crate::tuples::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Intersection {
    pub t: f32,
    pub object: Sphere,
}
pub struct Computations {
    pub t: f32,
    pub object: Sphere,
    pub point: TupleKind,
    pub eyev: TupleKind,
    pub normalv: TupleKind,
    pub inside: bool,
}
impl Intersection {
    pub fn prepare_computations(&self, ray: &Ray) -> Computations {
        let point = ray.position(self.t);
        let mut inside = false;
        let mut normalv = self.object.normal_at(&point);
        let eyev = -ray.direction;
        if normalv.dot(&eyev) < 0.0 {
            inside = true;
            normalv = -normalv;
        }
        Computations {
            t: self.t,
            object: self.object,
            point: point,
            eyev: eyev,
            normalv: normalv,
            inside: inside,
        }
    }
}
impl Eq for Intersection {}
impl Ord for Intersection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.t < other.t {
            return (std::cmp::Ordering::Less);
        } else if self.t > other.t {
            return (std::cmp::Ordering::Greater);
        }
        (std::cmp::Ordering::Equal)
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

#[derive(Debug, Clone, PartialEq)]
pub struct Intersections {
    pub intersections: Vec<Intersection>,
}

impl Intersections {
    pub fn new(xs: &[Intersection]) -> Self {
        let mut intersections = xs.to_vec();
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
    pub fn extend(&mut self, other: &Intersections) -> () {
        self.intersections.extend(other.intersections.clone());
        self.intersections.sort();
    }
    pub fn count(&self) -> usize {
        self.intersections.iter().count()
    }
}
impl Index<usize> for Intersections {
    type Output = Intersection;
    fn index(&self, index: usize) -> &Self::Output {
        &self.intersections[index]
    }
}
impl Intersection {
    pub const fn new(t: f32, object: Sphere) -> Self {
        Self { t, object }
    }
}

#[test]
fn an_intersection_encapsulates_t_and_object() {
    const S: Sphere = Sphere::unit();
    let i = Intersection::new(3.5, S);
    assert_eq!(i.t, 3.5);
    assert_eq!(i.object, S);
}
#[test]
fn aggregating_intersections() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(1.0, S);
    let i2 = Intersection::new(2.0, S);
    let xs = Intersections::new(&[i1, i2]);
    assert_eq!(xs[0].t, 1.0);
    assert_eq!(xs[1].t, 2.0);
}
#[test]
fn the_hit_when_all_intersections_have_positive_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(1.0, S);
    let i2 = Intersection::new(2.0, S);
    let xs = Intersections::new(&[i2, i1]);
    let i = xs.hit();
    assert_eq!(i.unwrap(), &i1);
}
#[test]
fn the_hit_when_some_intersections_have_negative_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(-1.0, S);
    let i2 = Intersection::new(1.0, S);
    let xs = Intersections::new(&[i2, i1]);
    let i = xs.hit();
    assert_eq!(i.unwrap(), &i2);
}
#[test]
fn the_hit_when_all_intersections_have_negative_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(-2.0, S);
    let i2 = Intersection::new(-1.0, S);
    let xs = Intersections::new(&[i2, i1]);
    let i = xs.hit();
    assert_eq!(i, None);
}
#[test]
fn the_hit_is_always_the_lowest_nonnegative_intersection() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(5.0, S);
    let i2 = Intersection::new(7.0, S);
    let i3 = Intersection::new(-3.0, S);
    let i4 = Intersection::new(2.0, S);
    let xs = Intersections::new(&[i1, i2, i3, i4]);
    let i = xs.hit();
    assert_eq!(i.unwrap(), &i4);
}
#[test]
fn precomputing_the_state_of_an_intersection() {
    let r = Ray::new(
        TupleKind::point(0.0, 0.0, -5.0),
        TupleKind::vector(0.0, 0.0, 1.0),
    );
    let shape = Sphere::unit();
    let i = Intersection::new(4.0, shape);
    let comps = i.prepare_computations(&r);
    assert_eq!(comps.t, i.t);
    assert_eq!(comps.object, i.object);
    assert_eq!(comps.point, TupleKind::point(0.0, 0.0, -1.0));
    assert_eq!(comps.eyev, TupleKind::vector(0.0, 0.0, -1.0));
    assert_eq!(comps.normalv, TupleKind::vector(0.0, 0.0, -1.0));
}
#[test]
fn the_hit_when_an_intersection_occurs_on_the_outside() {
    let r = Ray::new(
        TupleKind::point(0.0, 0.0, -5.0),
        TupleKind::vector(0.0, 0.0, 1.0),
    );
    let shape = Sphere::unit();
    let i = Intersection::new(4.0, shape);
    let comps = i.prepare_computations(&r);
    assert_eq!(comps.inside, false)
}
#[test]
fn the_hit_when_an_intersection_occurs_on_the_inside() {
    let r = Ray::new(
        TupleKind::point(0.0, 0.0, 0.0),
        TupleKind::vector(0.0, 0.0, 1.0),
    );
    let shape = Sphere::unit();
    let i = Intersection::new(1.0, shape);
    let comps = i.prepare_computations(&r);
    assert_eq!(comps.point, TupleKind::point(0.0, 0.0, 1.0));
    assert_eq!(comps.eyev, TupleKind::vector(0.0, 0.0, -1.0));
    assert_eq!(comps.inside, true);
    assert_eq!(comps.normalv, TupleKind::vector(0.0, 0.0, -1.0));
}
