use crate::rays::*;
use crate::spheres::*;
use crate::tuples::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Intersection {
    pub t: f32,
    pub object: Sphere,
}

impl Intersection {
    pub const fn new(t: f32, object: Sphere) -> Self {
        Self { t, object }
    }
}

pub fn intersections(xs: &[Intersection]) -> Vec<Intersection> {
    xs.to_vec()
}

pub fn hit(intersections: &Vec<Intersection>) -> Option<&Intersection> {
    let mut result = None;
    for intersection in intersections.iter() {
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
    let xs = intersections(&[i1, i2]);
    assert_eq!(xs[0].t, 1.0);
    assert_eq!(xs[1].t, 2.0);
}
#[test]
fn the_hit_when_all_intersections_have_positive_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(1.0, S);
    let i2 = Intersection::new(2.0, S);
    let xs = intersections(&[i2, i1]);
    let i = hit(&xs);
    assert_eq!(i.unwrap(), &i1);
}
#[test]
fn the_hit_when_some_intersections_have_negative_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(-1.0, S);
    let i2 = Intersection::new(1.0, S);
    let xs = intersections(&[i2, i1]);
    let i = hit(&xs);
    assert_eq!(i.unwrap(), &i2);
}
#[test]
fn the_hit_when_all_intersections_have_negative_t() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(-2.0, S);
    let i2 = Intersection::new(-1.0, S);
    let xs = intersections(&[i2, i1]);
    let i = hit(&xs);
    assert_eq!(i, None);
}
#[test]
fn the_hit_is_always_the_lowest_nonnegative_intersection() {
    const S: Sphere = Sphere::unit();
    let i1 = Intersection::new(5.0, S);
    let i2 = Intersection::new(7.0, S);
    let i3 = Intersection::new(-3.0, S);
    let i4 = Intersection::new(2.0, S);
    let xs = intersections(&[i1, i2, i3, i4]);
    let i = hit(&xs);
    assert_eq!(i.unwrap(), &i4);
}
