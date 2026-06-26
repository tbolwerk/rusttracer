// Constructive Solid Geometry combines two shapes with a set operation. Like a
// group, a CSG node owns no surface of its own; it owns two children (`left` and
// `right`, arena indices into `World::objects`) and a rule for which of their
// surface intersections survive. `World::intersect_object` intersects both
// children and then keeps only the allowed hits via `filter_intersections`. The
// node's data (operation, left, right, bounds) lives in the flat `Primitive`
// struct; only the operation enum and the rule helper remain here.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CsgOperation {
    Union,        // everything in either shape; the shared interior wall vanishes
    Intersection, // only the volume the two shapes share
    Difference,   // the left shape with the right carved out of it
}

// The heart of CSG: given the operation, whether the hit was on the left child
// (`lhit`), and whether the ray is currently inside the left/right child
// (`inl`/`inr`), decide whether this intersection lies on the combined surface.
pub fn intersection_allowed(op: CsgOperation, lhit: bool, inl: bool, inr: bool) -> bool {
    match op {
        // A left hit counts unless we are inside the right (that face is interior
        // to the union), and vice versa.
        CsgOperation::Union => (lhit && !inr) || (!lhit && !inl),
        // Keep a face only where it is inside the other shape.
        CsgOperation::Intersection => (lhit && inr) || (!lhit && inl),
        // Left faces survive outside the right; right faces survive only where
        // they bound the cavity, i.e. inside the left.
        CsgOperation::Difference => (lhit && !inr) || (!lhit && inl),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluating_the_rule_for_a_csg_operation() {
        use CsgOperation::*;
        // (operation, lhit, inl, inr) -> allowed, straight from the book's table.
        let cases = [
            (Union, true, true, true, false),
            (Union, true, true, false, true),
            (Union, true, false, true, false),
            (Union, true, false, false, true),
            (Union, false, true, true, false),
            (Union, false, true, false, false),
            (Union, false, false, true, true),
            (Union, false, false, false, true),
            (Intersection, true, true, true, true),
            (Intersection, true, true, false, false),
            (Intersection, true, false, true, true),
            (Intersection, true, false, false, false),
            (Intersection, false, true, true, true),
            (Intersection, false, true, false, true),
            (Intersection, false, false, true, false),
            (Intersection, false, false, false, false),
            (Difference, true, true, true, false),
            (Difference, true, true, false, true),
            (Difference, true, false, true, false),
            (Difference, true, false, false, true),
            (Difference, false, true, true, true),
            (Difference, false, true, false, true),
            (Difference, false, false, true, false),
            (Difference, false, false, false, false),
        ];
        for (op, lhit, inl, inr, expected) in cases {
            assert_eq!(
                intersection_allowed(op, lhit, inl, inr),
                expected,
                "op={op:?} lhit={lhit} inl={inl} inr={inr}"
            );
        }
    }

    use crate::intersections::*;
    use crate::rays::*;
    use crate::shapes::*;
    use crate::transformations::translation;
    use crate::tuples::*;
    use crate::worlds::World;

    #[test]
    fn csg_is_created_with_an_operation_and_two_shapes() {
        let mut w = World::new();
        let c = w.add_object(Primitive::csg(CsgOperation::Union));
        let s1 = w.add_object(Primitive::sphere());
        let s2 = w.add_object(Primitive::cube());
        w.set_csg_children(c, s1, s2);
        let csg = &w.objects[c];
        assert_eq!(csg.kind, ShapeKind::Csg);
        assert_eq!(csg.operation, CsgOperation::Union);
        assert_eq!(csg.left, Some(s1));
        assert_eq!(csg.right, Some(s2));
        // The children point back up at the CSG, as the book's `csg()` arranges.
        assert_eq!(w.objects[s1].parent(), Some(c));
        assert_eq!(w.objects[s2].parent(), Some(c));
    }

    #[test]
    fn filtering_a_list_of_intersections() {
        // (operation, surviving ts) — the book identifies the survivors by index
        // into xs; here those are t = 1,2,3,4 for indices 0,1,2,3.
        let cases = [
            (CsgOperation::Union, [1.0, 4.0]),
            (CsgOperation::Intersection, [2.0, 3.0]),
            (CsgOperation::Difference, [1.0, 2.0]),
        ];
        for (operation, expected) in cases {
            let mut w = World::new();
            let c = w.add_object(Primitive::csg(operation));
            let s1 = w.add_object(Primitive::sphere());
            let s2 = w.add_object(Primitive::cube());
            w.set_csg_children(c, s1, s2);
            let xs = Intersections::new(vec![
                Intersection::new(1.0, s1),
                Intersection::new(2.0, s2),
                Intersection::new(3.0, s1),
                Intersection::new(4.0, s2),
            ]);
            let result = w.filter_intersections(c, xs);
            assert_eq!(result.count(), 2, "op={operation:?}");
            assert_eq!(result[0].t, expected[0], "op={operation:?}");
            assert_eq!(result[1].t, expected[1], "op={operation:?}");
        }
    }

    #[test]
    fn a_ray_misses_a_csg_object() {
        let mut w = World::new();
        let c = w.add_object(Primitive::csg(CsgOperation::Union));
        let s1 = w.add_object(Primitive::sphere());
        let s2 = w.add_object(Primitive::cube());
        w.set_csg_children(c, s1, s2);
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 2.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        assert_eq!(w.intersect_object(c, &r).count(), 0);
    }

    #[test]
    fn a_ray_hits_a_csg_object() {
        let mut w = World::new();
        let c = w.add_object(Primitive::csg(CsgOperation::Union));
        let s1 = w.add_object(Primitive::sphere());
        let mut sphere2 = Primitive::sphere();
        sphere2.set_transform(translation(0.0, 0.0, 0.5));
        let s2 = w.add_object(sphere2);
        w.set_csg_children(c, s1, s2);
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
        let xs = w.intersect_object(c, &r);
        assert_eq!(xs.count(), 2);
        assert_eq!(xs[0].t, 4.0);
        assert_eq!(xs[0].object_id, s1);
        assert_eq!(xs[1].t, 6.5);
        assert_eq!(xs[1].object_id, s2);
    }
}
