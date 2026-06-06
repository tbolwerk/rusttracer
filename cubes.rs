use crate::intersections::*;
use crate::materials::*;
use crate::matrices::*;
use crate::rays::*;
use crate::shapes::*;
use crate::transformations::*;
use crate::tuples::*;

#[derive(Debug, PartialEq, Clone)]
pub struct Cube {
    pub transform: TransformData,
    material: Material,
}

impl Default for Cube {
    fn default() -> Self {
        Self {
            transform: TransformData::default(),
            material: Material::default(),
        }
    }
}

impl HasMaterial for Cube {
    fn set_material(&mut self, material: Material) -> () {
        self.material = material;
    }
    fn get_material(&self) -> Material {
        self.material.clone()
    }
}

impl Intersects for Cube {
    fn local_intersect(&self, ray: &Ray, object_id: usize) -> Intersections {
        Intersections::new(vec![])
    }
    fn local_normal_at(&self, point: &Point) -> Vector {
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}
