use crate::intersections::*;
use crate::materials::*;
use crate::matrices::*;
use crate::rays::*;
use crate::shapes::*;
use crate::tuples::*;

impl Default for Plane {
    fn default() -> Self {
        Self {
            transform: Matrix::identity(),
            inverse_transform: None,
            material: Material::default(),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct Plane {
    transform: Matrix<4, 4>,
    inverse_transform: Option<Matrix<4, 4>>,
    material: Material,
}
impl HasTransform for Plane {
    fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse_transform = inverse(&self.transform);
    }
    fn get_inverse_transform(&self) -> Option<Matrix<4, 4>> {
        self.inverse_transform
    }
    fn get_transform(&self) -> Matrix<4, 4> {
        self.transform
    }
}
impl HasMaterial for Plane {
    fn set_material(&mut self, material: Material) -> () {
        self.material = material;
    }
    fn get_material(&self) -> Material {
        self.material.clone()
    }
}
impl Intersects for Plane {
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        if ray.direction.y().abs() < EPSILON {
            return Intersections::new(vec![]);
        }
        Intersections::new(vec![Intersection {
            t: -ray.origin.y / ray.direction.y,
            object_id: object_id,
        }])
    }
    fn local_normal_at(&self, _: &Point) -> Vector {
        Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }
}
mod tests {
    use super::*;
    #[test]
    fn the_normal_of_a_plane_is_constant_everywhere() {
        let p = Plane::default();
        let n1 = p.local_normal_at(&Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
        let n2 = p.local_normal_at(&Point {
            x: 10.0,
            y: 0.0,
            z: -10.0,
        });
        let n3 = p.local_normal_at(&Point {
            x: -5.0,
            y: 0.0,
            z: 150.0,
        });
        assert_eq!(
            n1,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
        assert_eq!(
            n2,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
        assert_eq!(
            n3,
            Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            }
        );
    }
    #[test]
    fn intersect_with_a_ray_parallel_to_the_plane() {
        let p = Plane::default();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        };
        let xs = p.local_intersect(&r, 0);
        assert_eq!(xs.count(), 0);
    }
    #[test]
    fn intersect_with_a_coplanar_ray() {
        let p = Plane::default();
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
        let xs = p.local_intersect(&r, 0);
        assert_eq!(xs.count(), 0);
    }
    #[test]
    fn a_plane_intersecting_a_plane_from_above() {
        let p = Shape::plane();
        let r = Ray {
            origin: Point {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            direction: Vector {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
        };
        let xs = p.intersect(&r, 0);
        assert_eq!(xs.count(), 1);
        assert_eq!(xs[0].t, 1.0);
        assert_eq!(xs[0].object_id, 0);
    }
}
