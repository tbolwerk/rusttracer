use std::ops::Index;

use crate::intersections;
use crate::rays::*;
use crate::spheres::*;
use crate::tuples::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Intersection<'a> {
    pub t: f32,
    pub object: &'a Sphere,
}
pub struct Computations<'a> {
    pub t: f32,
    pub object: &'a Sphere,
    pub point: Point,
    pub eyev: Vector,
    pub normalv: Vector,
    pub inside: bool,
    pub over_point: Point,
}
impl<'a> Intersection<'a> {
    pub fn prepare_computations(&self, ray: &Ray) -> Computations<'a> {
        let point = ray.position(self.t);
        let mut inside = false;
        let mut normalv = self.object.normal_at(&point);
        let eyev = -ray.direction.clone();
        if normalv.dot(&eyev) < 0.0 {
            inside = true;
            normalv = -normalv;
        }
        let over_point = point.clone() + normalv.clone() * EPSILON;
        Computations {
            t: self.t,
            object: self.object,
            point: point,
            eyev: eyev,
            normalv: normalv,
            inside: inside,
            over_point: over_point,
        }
    }
}
impl<'a> Eq for Intersection<'a> {}
impl<'a> Ord for Intersection<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.t < other.t {
            return std::cmp::Ordering::Less;
        } else if self.t > other.t {
            return std::cmp::Ordering::Greater;
        }
        std::cmp::Ordering::Equal
    }
}
impl<'a> PartialOrd for Intersection<'a> {
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
pub struct Intersections<'a> {
    pub intersections: Vec<Intersection<'a>>,
}

impl<'a> Intersections<'a> {
    pub fn new(xs: Vec<Intersection<'a>>) -> Self {
        let mut intersections = xs;
        intersections.sort();
        Self { intersections }
    }

    pub fn hit(&self) -> Option<&Intersection<'a>> {
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
    pub fn extend(&mut self, mut other: Intersections<'a>) -> () {
        self.intersections.append(&mut other.intersections);
        self.intersections.sort();
    }
    pub fn count(&self) -> usize {
        self.intersections.len()
    }
}
impl<'a> Index<usize> for Intersections<'a> {
    type Output = Intersection<'a>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.intersections[index]
    }
}
impl<'a> Intersection<'a> {
    pub const fn new(t: f32, object: &'a Sphere) -> Self {
        Self { t, object }
    }
}

#[test]
fn an_intersection_encapsulates_t_and_object() {
    const S: Sphere = Sphere::unit();
    let i = Intersection::new(3.5, &S);
    assert_eq!(i.t, 3.5);
    assert_eq!(i.object, &S);
}
#[test]
fn aggregating_intersections() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(1.0, &S);
    let i2 = Intersection::new(2.0, &S);
    let xs = Intersections::new(vec![i1, i2]);
    assert_eq!(xs[0].t, 1.0);
    assert_eq!(xs[1].t, 2.0);
}
#[test]
fn the_hit_when_all_intersections_have_positive_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(1.0, &S);
    let i2 = Intersection::new(2.0, &S);
    let xs = Intersections::new(vec![i2, i1]);
    let i = xs.hit();
    assert_eq!(i.unwrap(), &i1);
}
#[test]
fn the_hit_when_some_intersections_have_negative_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(-1.0, &S);
    let i2 = Intersection::new(1.0, &S);
    let xs = Intersections::new(vec![i2, i1]);
    let i = xs.hit();
    assert_eq!(i.unwrap(), &i2);
}
#[test]
fn the_hit_when_all_intersections_have_negative_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(-2.0, &S);
    let i2 = Intersection::new(-1.0, &S);
    let xs = Intersections::new(vec![i2, i1]);
    let i = xs.hit();
    assert_eq!(i, None);
}
#[test]
fn the_hit_is_always_the_lowest_nonnegative_intersection() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(5.0, &S);
    let i2 = Intersection::new(7.0, &S);
    let i3 = Intersection::new(-3.0, &S);
    let i4 = Intersection::new(2.0, &S);
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
    let shape = Sphere::unit();
    let i = Intersection::new(4.0, &shape);
    let comps = i.prepare_computations(&r);
    assert_eq!(comps.t, i.t);
    assert_eq!(comps.object, i.object);
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
    let shape = Sphere::unit();
    let i = Intersection::new(4.0, &shape);
    let comps = i.prepare_computations(&r);
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
    let shape = Sphere::unit();
    let i = Intersection::new(1.0, &shape);
    let comps = i.prepare_computations(&r);
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
