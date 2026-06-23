use crate::matrices::Matrix;
use crate::rays::Ray;
use crate::tuples::*;

// An axis-aligned bounding box (AABB) used to accelerate group intersection.
// A group can test a ray against its enclosing box first and, on a miss, skip
// every child at once instead of intersecting each primitive. This is the
// optimization from the book's "Bounding boxes and hierarchies" bonus chapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Point,
    pub max: Point,
}

impl BoundingBox {
    // An empty box: min at +inf and max at -inf, so the first point added
    // defines the real extent in every axis.
    pub fn empty() -> Self {
        Self {
            min: Point {
                x: Number::INFINITY,
                y: Number::INFINITY,
                z: Number::INFINITY,
            },
            max: Point {
                x: Number::NEG_INFINITY,
                y: Number::NEG_INFINITY,
                z: Number::NEG_INFINITY,
            },
        }
    }

    pub fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    // Grow the box so it contains `p`.
    pub fn add_point(&mut self, p: Point) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }

    // Grow the box so it contains all of `other`.
    pub fn add_box(&mut self, other: &BoundingBox) {
        self.add_point(other.min);
        self.add_point(other.max);
    }

    // Transform the eight corners by `m` and return the AABB that encloses
    // them. A rotated box is no longer axis-aligned, so we re-fit a new box
    // around the transformed corners (it may be looser than the original).
    pub fn transform(&self, m: Matrix<4, 4>) -> BoundingBox {
        let corners = [
            self.min,
            Point {
                x: self.min.x,
                y: self.min.y,
                z: self.max.z,
            },
            Point {
                x: self.min.x,
                y: self.max.y,
                z: self.min.z,
            },
            Point {
                x: self.min.x,
                y: self.max.y,
                z: self.max.z,
            },
            Point {
                x: self.max.x,
                y: self.min.y,
                z: self.min.z,
            },
            Point {
                x: self.max.x,
                y: self.min.y,
                z: self.max.z,
            },
            Point {
                x: self.max.x,
                y: self.max.y,
                z: self.min.z,
            },
            self.max,
        ];
        let mut out = BoundingBox::empty();
        for c in corners {
            out.add_point(m * c);
        }
        out
    }

    // Is `p` inside (or on the surface of) this box?
    pub fn contains_point(&self, p: Point) -> bool {
        self.min.x <= p.x
            && p.x <= self.max.x
            && self.min.y <= p.y
            && p.y <= self.max.y
            && self.min.z <= p.z
            && p.z <= self.max.z
    }

    // Does this box fully contain `other`? True when both of `other`'s corners
    // lie inside, which (for axis-aligned boxes) means all of it does.
    pub fn contains_box(&self, other: &BoundingBox) -> bool {
        self.contains_point(other.min) && self.contains_point(other.max)
    }

    // Split the box in half across its longest axis, returning the (lower, upper)
    // halves. Used by `World::divide` to partition a group's children, the core
    // of the "Bounding Boxes and Hierarchies" subdivision.
    pub fn split(&self) -> (BoundingBox, BoundingBox) {
        let dx = self.max.x - self.min.x;
        let dy = self.max.y - self.min.y;
        let dz = self.max.z - self.min.z;
        let greatest = dx.max(dy).max(dz);

        let (mut x0, mut y0, mut z0) = (self.min.x, self.min.y, self.min.z);
        let (mut x1, mut y1, mut z1) = (self.max.x, self.max.y, self.max.z);
        if greatest == dx {
            x0 = x0 + dx / 2.0;
            x1 = x0;
        } else if greatest == dy {
            y0 = y0 + dy / 2.0;
            y1 = y0;
        } else {
            z0 = z0 + dz / 2.0;
            z1 = z0;
        }

        let mid_min = Point { x: x0, y: y0, z: z0 };
        let mid_max = Point { x: x1, y: y1, z: z1 };
        (
            BoundingBox::new(self.min, mid_max),
            BoundingBox::new(mid_min, self.max),
        )
    }

    // Does the ray's line pass through the box? Same slab test as `Cube`, but
    // against arbitrary min/max bounds. Used only to cull, so it answers the
    // yes/no question and does not return intersection points.
    pub fn intersects(&self, ray: &Ray) -> bool {
        fn check_axis(origin: Number, direction: Number, min: Number, max: Number) -> (Number, Number) {
            let tmin_numerator = min - origin;
            let tmax_numerator = max - origin;
            // Mirror Cube::local_intersect: multiply by Number::MAX rather than
            // INFINITY when the ray is parallel to a slab, so a numerator of 0
            // stays 0 instead of becoming NaN.
            let (tmin, tmax) = if direction.abs() >= EPSILON {
                (tmin_numerator / direction, tmax_numerator / direction)
            } else {
                (tmin_numerator * Number::MAX, tmax_numerator * Number::MAX)
            };
            if tmin > tmax {
                (tmax, tmin)
            } else {
                (tmin, tmax)
            }
        }

        let (xtmin, xtmax) = check_axis(ray.origin.x, ray.direction.x, self.min.x, self.max.x);
        let (ytmin, ytmax) = check_axis(ray.origin.y, ray.direction.y, self.min.y, self.max.y);
        let (ztmin, ztmax) = check_axis(ray.origin.z, ray.direction.z, self.min.z, self.max.z);

        let tmin = xtmin.max(ytmin).max(ztmin);
        let tmax = xtmax.min(ytmax).min(ztmax);

        tmin <= tmax
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformations::*;

    #[test]
    fn an_empty_box_has_inverted_bounds() {
        let b = BoundingBox::empty();
        assert_eq!(b.min.x, Number::INFINITY);
        assert_eq!(b.max.x, Number::NEG_INFINITY);
    }

    #[test]
    fn adding_points_grows_the_box() {
        let mut b = BoundingBox::empty();
        b.add_point(Point {
            x: -5.0,
            y: 2.0,
            z: 0.0,
        });
        b.add_point(Point {
            x: 7.0,
            y: 0.0,
            z: -3.0,
        });
        assert_eq!(
            b.min,
            Point {
                x: -5.0,
                y: 0.0,
                z: -3.0
            }
        );
        assert_eq!(
            b.max,
            Point {
                x: 7.0,
                y: 2.0,
                z: 0.0
            }
        );
    }

    #[test]
    fn a_ray_intersects_a_bounding_box() {
        let b = BoundingBox::new(
            Point {
                x: -1.0,
                y: -1.0,
                z: -1.0,
            },
            Point {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        );
        let hit = Ray {
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
        let miss = Ray {
            origin: Point {
                x: 0.0,
                y: 5.0,
                z: -5.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        assert!(b.intersects(&hit));
        assert!(!b.intersects(&miss));
    }

    #[test]
    fn splitting_a_perfect_cube() {
        let b = BoundingBox::new(
            Point { x: -1.0, y: -4.0, z: -5.0 },
            Point { x: 9.0, y: 6.0, z: 5.0 },
        );
        let (left, right) = b.split();
        assert_eq!(left.min, Point { x: -1.0, y: -4.0, z: -5.0 });
        assert_eq!(left.max, Point { x: 4.0, y: 6.0, z: 5.0 });
        assert_eq!(right.min, Point { x: 4.0, y: -4.0, z: -5.0 });
        assert_eq!(right.max, Point { x: 9.0, y: 6.0, z: 5.0 });
    }

    #[test]
    fn splitting_an_x_wide_box() {
        let b = BoundingBox::new(
            Point { x: -1.0, y: -2.0, z: -3.0 },
            Point { x: 9.0, y: 5.5, z: 3.0 },
        );
        let (left, right) = b.split();
        assert_eq!(left.min, Point { x: -1.0, y: -2.0, z: -3.0 });
        assert_eq!(left.max, Point { x: 4.0, y: 5.5, z: 3.0 });
        assert_eq!(right.min, Point { x: 4.0, y: -2.0, z: -3.0 });
        assert_eq!(right.max, Point { x: 9.0, y: 5.5, z: 3.0 });
    }

    #[test]
    fn splitting_a_y_wide_box() {
        let b = BoundingBox::new(
            Point { x: -1.0, y: -2.0, z: -3.0 },
            Point { x: 5.0, y: 8.0, z: 3.0 },
        );
        let (left, right) = b.split();
        assert_eq!(left.max, Point { x: 5.0, y: 3.0, z: 3.0 });
        assert_eq!(right.min, Point { x: -1.0, y: 3.0, z: -3.0 });
    }

    #[test]
    fn splitting_a_z_wide_box() {
        let b = BoundingBox::new(
            Point { x: -1.0, y: -2.0, z: -3.0 },
            Point { x: 5.0, y: 3.0, z: 7.0 },
        );
        let (left, right) = b.split();
        assert_eq!(left.max, Point { x: 5.0, y: 3.0, z: 2.0 });
        assert_eq!(right.min, Point { x: -1.0, y: -2.0, z: 2.0 });
    }

    #[test]
    fn a_box_contains_a_point_and_a_box() {
        let b = BoundingBox::new(
            Point { x: 5.0, y: -2.0, z: 0.0 },
            Point { x: 11.0, y: 4.0, z: 7.0 },
        );
        assert!(b.contains_point(Point { x: 5.0, y: -2.0, z: 0.0 }));
        assert!(b.contains_point(Point { x: 8.0, y: 1.0, z: 3.0 }));
        assert!(!b.contains_point(Point { x: 3.0, y: 0.0, z: 3.0 }));
        assert!(b.contains_box(&BoundingBox::new(
            Point { x: 6.0, y: -1.0, z: 1.0 },
            Point { x: 10.0, y: 3.0, z: 6.0 },
        )));
        assert!(!b.contains_box(&BoundingBox::new(
            Point { x: 4.0, y: -3.0, z: -1.0 },
            Point { x: 10.0, y: 3.0, z: 6.0 },
        )));
    }

    #[test]
    fn transforming_a_bounding_box_refits_it() {
        let b = BoundingBox::new(
            Point {
                x: -1.0,
                y: -1.0,
                z: -1.0,
            },
            Point {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        );
        let moved = b.transform(translation(5.0, 0.0, 0.0));
        assert_eq!(
            moved.min,
            Point {
                x: 4.0,
                y: -1.0,
                z: -1.0
            }
        );
        assert_eq!(
            moved.max,
            Point {
                x: 6.0,
                y: 1.0,
                z: 1.0
            }
        );
    }
}
