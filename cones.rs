use crate::intersections::*;
use crate::materials::*;
use crate::rays::*;
use crate::shapes::*;
use crate::tuples::*;

#[derive(Debug, PartialEq, Clone)]
pub struct Cone {
    pub transform: TransformData,
    material: Material,
    pub minimum: Number,
    pub maximum: Number,
    pub closed: bool,
}

impl Cone {
    pub fn new(minimum: Number, maximum: Number, closed: bool) -> Self {
        Cone {
            transform: TransformData::default(),
            material: Material::default(),
            minimum,
            maximum,
            closed,
        }
    }
    fn intersect_caps(&self, ray: &Ray, object_id: usize, xs: &mut Vec<Intersection>) {
        if !self.closed || almost_eq(ray.direction.y(), 0.0) {
            return;
        }

        fn check_caps(ray: &Ray, t: Number) -> bool {
            let x = ray.origin.x() + t * ray.direction.x();
            let z = ray.origin.z() + t * ray.direction.z();

            (x.powf(2.0) + z.powf(2.0)) <= 1.0 + EPSILON
        }

        let bounds = [self.minimum, self.maximum];
        for bound in bounds {
            let t = (bound - ray.origin.y()) / ray.direction.y();
            if check_caps(ray, t) {
                xs.push(Intersection::new(t, object_id));
            }
        }
    }
}

impl Default for Cone {
    fn default() -> Self {
        Self {
            transform: TransformData::default(),
            material: Material::default(),
            minimum: Number::MIN,
            maximum: Number::MAX,
            closed: false,
        }
    }
}

impl HasMaterial for Cone {
    fn set_material(&mut self, material: Material) -> () {
        self.material = material;
    }
    fn get_material(&self) -> Material {
        self.material.clone()
    }
}

impl Intersects for Cone {
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        let a = ray.direction.x().powi(2) - ray.direction.y().powi(2) + ray.direction.z().powi(2);

        let mut xs: Vec<Intersection> = vec![];

        let b = 2.0 * ray.origin.x() * ray.direction.x() - 2.0 * ray.origin.y() * ray.direction.y()
            + 2.0 * ray.origin.z() * ray.direction.z();

        let c = ray.origin.x().powi(2) - ray.origin.y().powi(2) + ray.origin.z().powi(2);

        if almost_eq(a, 0.0) && !almost_eq(b, 0.0) {
            let t = -c / (2.0 * b);
            xs.push(Intersection::new(t, object_id));
            return Intersections::new(xs);
        }

        let disc = b.powi(2) - 4.0 * a * c;

        // A truly missing ray has a clearly negative discriminant. A tangent ray
        // gives disc == 0 in exact math, but f32 rounding can push it slightly
        // negative, so tolerate that and clamp before taking the square root.
        if disc < -EPSILON {
            return Intersections::new(xs);
        }
        let disc = disc.max(0.0);

        let mut t0 = (-b - sqrt(disc)) / (2.0 * a);
        let mut t1 = (-b + sqrt(disc)) / (2.0 * a);

        if t0 > t1 {
            (t0, t1) = (t1, t0);
        }

        for t in [t0, t1] {
            let y = ray.origin.y() + t * ray.direction.y();
            if self.minimum < y && y < self.maximum {
                xs.push(Intersection::new(t, object_id));
            }
        }

        self.intersect_caps(ray, object_id, &mut xs);

        Intersections::new(xs)
    }
    fn local_normal_at(&self, point: &Point) -> Vector {
        let dist = point.x().powi(2) + point.z().powi(2);

        if dist < 1.0 + EPSILON && point.y() >= self.maximum - EPSILON {
            return Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            };
        }
        if dist < 1.0 + EPSILON && point.y() <= self.minimum + EPSILON {
            return Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            };
        }
        let mut y = (point.x().powi(2) + point.z().powi(2)).sqrt();
        if point.y() > 0.0 {
            y = -y;
        }
        Vector {
            x: point.x(),
            y,
            z: point.z(),
        }
    }
}

#[test]
fn intersecting_a_cone_with_a_ray() {
    let shape = Cone::default();
    struct Example {
        origin: Point,
        direction: Vector,
        t0: Number,
        t1: Number,
    }
    let examples = [
        Example {
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
            t0: 5.0,
            t1: 5.0,
        },
        Example {
            origin: Point {
                x: 0.0,
                y: 0.0,
                z: -5.0,
            },
            direction: Vector {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            t0: 8.66025,
            t1: 8.66025,
        },
        Example {
            origin: Point {
                x: 1.0,
                y: 1.0,
                z: -5.0,
            },
            direction: Vector {
                x: -0.5,
                y: -1.0,
                z: 1.0,
            },
            t0: 4.55006,
            t1: 49.44994,
        },
    ];
    for Example {
        origin,
        direction,
        t0,
        t1,
    } in examples
    {
        let dir = direction.normalize();
        let r = Ray {
            origin,
            direction: dir,
        };
        let xs = shape.local_intersect(&r, 0);
        assert_eq!(xs.count(), 2);
        assert_almost_eq!(xs[0].t, t0);
        assert_almost_eq!(xs[1].t, t1);
    }
}
#[test]
fn intersecting_a_cone_with_a_ray_parallel_to_one_of_its_halves() {
    let shape = Cone::default();
    let direction = Vector {
        x: 0.0,
        y: 1.0,
        z: 1.0,
    }
    .normalize();
    let r = Ray {
        origin: Point {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        direction,
    };
    let xs = shape.local_intersect(&r, 0);
    assert_eq!(xs.count(), 1);
    assert_almost_eq!(xs[0].t, 0.35355);
}
#[test]
fn computing_the_normal_vector_on_a_cone() {
    let shape = Cone::default();
    struct Example {
        point: Point,
        normal: Vector,
    }
    let examples = [
        Example {
            point: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            normal: Vector {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
        Example {
            point: Point {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            normal: Vector {
                x: 1.0,
                y: -sqrt(2.0),
                z: 1.0,
            },
        },
        Example {
            point: Point {
                x: -1.0,
                y: -1.0,
                z: 0.0,
            },
            normal: Vector {
                x: -1.0,
                y: 1.0,
                z: 0.0,
            },
        },
    ];
    for Example { point, normal } in examples {
        let n = shape.local_normal_at(&point);
        assert_almost_eq!(n.x, normal.x);
        assert_almost_eq!(n.y, normal.y);
        assert_almost_eq!(n.z, normal.z);
    }
}
