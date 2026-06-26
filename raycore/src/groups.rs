// A group is an interior node of the shape hierarchy. It owns no geometry of
// its own; it just transforms the space its children live in. Children are
// referenced by their index (`object_id`) into `World::objects`, the same arena
// the rest of the renderer addresses shapes through, and now live in the flat
// `Primitive` struct (its `children` and `bounds` fields). A group has no
// surface: rays are dispatched to its children by `World::intersect_object` and
// normals are resolved on the hit leaf by `World::normal_at`, so there are no
// per-shape intersect/normal functions here.

#[cfg(test)]
mod tests {
    use crate::matrices::*;
    use crate::rays::*;
    use crate::shapes::*;
    use crate::transformations::*;
    use crate::tuples::*;
    use crate::worlds::*;

    #[test]
    fn creating_a_new_group() {
        let g = Primitive::group();
        assert_eq!(g.get_transform(), Matrix::identity());
        assert_eq!(g.parent(), None);
        assert_eq!(g.kind, ShapeKind::Group);
        assert_eq!(g.child_count, 0);
    }

    #[test]
    fn adding_a_child_to_a_group() {
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
        let s = w.add_child(g, Primitive::sphere());
        // child records the group as parent, group records the child's id
        assert_eq!(w.objects[s].parent(), Some(g));
        assert_eq!(w.objects[g].kind, ShapeKind::Group);
        assert_eq!(w.children[g], vec![s]);
    }

    #[test]
    fn intersecting_a_ray_with_an_empty_group() {
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
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
        let g = w.add_object(Primitive::group());
        let s1 = w.add_child(g, Primitive::sphere());
        let mut sphere2 = Primitive::sphere();
        sphere2.set_transform(translation(0.0, 0.0, -3.0));
        let s2 = w.add_child(g, sphere2);
        let mut sphere3 = Primitive::sphere();
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
        let mut xs = w.intersect_object(g, &r);
        assert_eq!(xs.count(), 4);
        // `intersect_object` no longer sorts internally (the top-level
        // `intersect_world` does); sort here to check the by-t ordering.
        xs.sort();
        // ordered by t: the two hits on s2 (nearer) precede the two on s1
        assert_eq!(xs[0].object_id, s2);
        assert_eq!(xs[1].object_id, s2);
        assert_eq!(xs[2].object_id, s1);
        assert_eq!(xs[3].object_id, s1);
    }

    #[test]
    fn intersecting_a_transformed_group() {
        let mut w = World::new();
        let mut group = Primitive::group();
        group.set_transform(scaling(2.0, 2.0, 2.0));
        let g = w.add_object(group);
        let mut sphere = Primitive::sphere();
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
    fn computing_bounds_sets_a_group_box_around_its_children() {
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
        let mut s = Primitive::sphere();
        s.set_transform(translation(3.0, 0.0, 0.0));
        w.add_child(g, s);
        w.compute_bounds();
        // unit sphere at x=3 spans x in [2,4], y and z in [-1,1]
        let bounds = w.objects[g].bounds().expect("bounds should be computed");
        assert_eq!(
            bounds.min,
            Point {
                x: 2.0,
                y: -1.0,
                z: -1.0
            }
        );
        assert_eq!(
            bounds.max,
            Point {
                x: 4.0,
                y: 1.0,
                z: 1.0
            }
        );
    }

    #[test]
    fn culling_never_changes_intersection_results() {
        // Build a small cluster of spheres inside a group.
        let mut w = World::new();
        let g = w.add_object(Primitive::group());
        for x in [-3.0, 0.0, 3.0] {
            let mut s = Primitive::sphere();
            s.set_transform(translation(x, 0.0, 0.0));
            w.add_child(g, s);
        }
        w.compute_bounds();
        let mut w_off = w.clone();
        w_off.use_bounds = false;

        // A ray straight through the middle sphere: culling must not drop hits.
        let through = Ray {
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
        assert_eq!(
            w.intersect_object(g, &through).count(),
            w_off.intersect_object(g, &through).count()
        );

        // A ray well above the cluster: culled to nothing, and brute force
        // agrees there is nothing to hit.
        let over = Ray {
            origin: Point {
                x: 0.0,
                y: 10.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        assert_eq!(w.intersect_object(g, &over).count(), 0);
        assert_eq!(w_off.intersect_object(g, &over).count(), 0);
    }

    #[test]
    fn converting_a_point_from_world_to_object_space() {
        let mut w = World::new();
        let mut g1 = Primitive::group();
        g1.set_transform(rotation_y(PI / 2.0));
        let g1 = w.add_object(g1);
        let mut g2 = Primitive::group();
        g2.set_transform(scaling(2.0, 2.0, 2.0));
        let g2 = w.add_child(g1, g2);
        let mut sphere = Primitive::sphere();
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
        let mut g1 = Primitive::group();
        g1.set_transform(rotation_y(PI / 2.0));
        let g1 = w.add_object(g1);
        let mut g2 = Primitive::group();
        g2.set_transform(scaling(1.0, 2.0, 3.0));
        let g2 = w.add_child(g1, g2);
        let mut sphere = Primitive::sphere();
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
