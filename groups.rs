use crate::intersections::Intersections;
use crate::materials::Material;
use crate::rays::Ray;
use crate::shapes::{HasMaterial, Intersects, TransformData};

// A group is an interior node of the shape hierarchy. It owns no geometry of
// its own; it just transforms the space its children live in. Children are
// referenced by their index (`object_id`) into `World::objects`, the same
// arena the rest of the renderer addresses shapes through. This is the
// index-based equivalent of the book's parent/child pointers.
#[derive(Debug, PartialEq, Clone)]
pub struct Group {
    pub transform: TransformData,
    pub children: Vec<usize>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            transform: TransformData::default(),
            children: vec![],
        }
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::new()
    }
}

// A group has no surface, so these never run for a group in practice: rays are
// dispatched to its children by `World::intersect_object` and normals are
// resolved on the hit leaf by `World::normal_at`. The impls exist only so
// `Group` can sit in the `Shape` enum alongside the primitives.
impl Intersects for Group {
    fn local_intersect(&self, _ray: &Ray, _object_id: usize) -> Intersections {
        unreachable!("groups are intersected through World::intersect_object")
    }
}

impl HasMaterial for Group {
    fn set_material(&mut self, _material: Material) {}
    fn get_material(&self) -> Material {
        Material::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrices::*;
    use crate::shapes::*;
    use crate::transformations::*;
    use crate::tuples::*;
    use crate::worlds::*;

    #[test]
    fn creating_a_new_group() {
        let g = Shape::group();
        assert_eq!(g.get_transform(), Matrix::identity());
        assert_eq!(g.parent(), None);
        match g {
            Shape::Group(group) => assert_eq!(group.children, Vec::<usize>::new()),
            _ => panic!("expected a group"),
        }
    }

    #[test]
    fn adding_a_child_to_a_group() {
        let mut w = World::new();
        let g = w.add_object(Shape::group());
        let s = w.add_child(g, Shape::sphere());
        // child records the group as parent, group records the child's id
        assert_eq!(w.objects[s].parent(), Some(g));
        match &w.objects[g] {
            Shape::Group(group) => assert_eq!(group.children, vec![s]),
            _ => panic!("expected a group"),
        }
    }

    #[test]
    fn intersecting_a_ray_with_an_empty_group() {
        let mut w = World::new();
        let g = w.add_object(Shape::group());
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
        let xs = w.intersect_object(g, &r);
        assert_eq!(xs.count(), 0);
    }

    #[test]
    fn intersecting_a_ray_with_a_nonempty_group() {
        let mut w = World::new();
        let g = w.add_object(Shape::group());
        let s1 = w.add_child(g, Shape::sphere());
        let mut sphere2 = Shape::sphere();
        sphere2.set_transform(translation(0.0, 0.0, -3.0));
        let s2 = w.add_child(g, sphere2);
        let mut sphere3 = Shape::sphere();
        sphere3.set_transform(translation(5.0, 0.0, 0.0));
        let _s3 = w.add_child(g, sphere3);
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
        let xs = w.intersect_object(g, &r);
        assert_eq!(xs.count(), 4);
        // ordered by t: the two hits on s2 (nearer) precede the two on s1
        assert_eq!(xs[0].object_id, s2);
        assert_eq!(xs[1].object_id, s2);
        assert_eq!(xs[2].object_id, s1);
        assert_eq!(xs[3].object_id, s1);
    }

    #[test]
    fn intersecting_a_transformed_group() {
        let mut w = World::new();
        let mut group = Shape::group();
        group.set_transform(scaling(2.0, 2.0, 2.0));
        let g = w.add_object(group);
        let mut sphere = Shape::sphere();
        sphere.set_transform(translation(5.0, 0.0, 0.0));
        w.add_child(g, sphere);
        let r = Ray {
            origin: Point {
                x: 10.0,
                y: 0.0,
                z: -10.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = w.intersect_object(g, &r);
        assert_eq!(xs.count(), 2);
    }

    #[test]
    fn converting_a_point_from_world_to_object_space() {
        let mut w = World::new();
        let mut g1 = Shape::group();
        g1.set_transform(rotation_y(PI / 2.0));
        let g1 = w.add_object(g1);
        let mut g2 = Shape::group();
        g2.set_transform(scaling(2.0, 2.0, 2.0));
        let g2 = w.add_child(g1, g2);
        let mut sphere = Shape::sphere();
        sphere.set_transform(translation(5.0, 0.0, 0.0));
        let s = w.add_child(g2, sphere);
        let p = w.world_to_object(
            s,
            Point {
                x: -2.0,
                y: 0.0,
                z: -10.0,
            },
        );
        assert_eq!(
            p,
            Point {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
    }

    #[test]
    fn finding_the_normal_on_a_child_object() {
        let mut w = World::new();
        let mut g1 = Shape::group();
        g1.set_transform(rotation_y(PI / 2.0));
        let g1 = w.add_object(g1);
        let mut g2 = Shape::group();
        g2.set_transform(scaling(1.0, 2.0, 3.0));
        let g2 = w.add_child(g1, g2);
        let mut sphere = Shape::sphere();
        sphere.set_transform(translation(5.0, 0.0, 0.0));
        let s = w.add_child(g2, sphere);
        let n = w.normal_at(
            s,
            Point {
                x: 1.7321,
                y: 1.1547,
                z: -5.5774,
            },
        );
        let expected = Vector {
            x: 0.2857,
            y: 0.4286,
            z: -0.8571,
        };
        assert!(
            (n.x - expected.x).abs() < 1e-3,
            "x: {} vs {}",
            n.x,
            expected.x
        );
        assert!(
            (n.y - expected.y).abs() < 1e-3,
            "y: {} vs {}",
            n.y,
            expected.y
        );
        assert!(
            (n.z - expected.z).abs() < 1e-3,
            "z: {} vs {}",
            n.z,
            expected.z
        );
    }
}
