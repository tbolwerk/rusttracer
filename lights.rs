use crate::tuples::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PointLight {
    pub position: Point,
    pub intensity: Color,
}

// A rectangular area light (the "Area Light and Soft Shadows" bonus chapter).
// Instead of one point, the emitter is a grid of `usteps` x `vsteps` cells
// spanning the rectangle `corner + uvec_full + vvec_full`. Shadow rays are cast
// to a point in each cell and averaged, so an occluder casts a soft penumbra
// rather than a hard edge. `uvec`/`vvec` are the per-cell step vectors.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaLight {
    pub corner: Point,
    pub uvec: Vector,
    pub usteps: usize,
    pub vvec: Vector,
    pub vsteps: usize,
    pub samples: usize,
    pub position: Point, // the rectangle's center, used where a single point is needed
    pub intensity: Color,
}

impl AreaLight {
    pub fn new(
        corner: Point,
        full_uvec: Vector,
        usteps: usize,
        full_vvec: Vector,
        vsteps: usize,
        intensity: Color,
    ) -> Self {
        Self {
            corner,
            uvec: full_uvec * (1.0 / usteps as Number),
            usteps,
            vvec: full_vvec * (1.0 / vsteps as Number),
            vsteps,
            samples: usteps * vsteps,
            position: corner + full_uvec * 0.5 + full_vvec * 0.5,
            intensity,
        }
    }
    // The center of cell (u, v). Sampling cell centers (the +0.5 offset) gives a
    // fixed, deterministic pattern; the book optionally jitters within each cell
    // for smoother penumbras, which is omitted here so renders stay reproducible
    // across the parallel renderer.
    pub fn point_on_light(&self, u: usize, v: usize) -> Point {
        self.corner + self.uvec * (u as Number + 0.5) + self.vvec * (v as Number + 0.5)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Light {
    Point(PointLight),
    Area(AreaLight),
}

impl Light {
    pub const fn point_light(position: Point, intensity: Color) -> Light {
        Light::Point(PointLight {
            position,
            intensity,
        })
    }
    pub fn area_light(
        corner: Point,
        full_uvec: Vector,
        usteps: usize,
        full_vvec: Vector,
        vsteps: usize,
        intensity: Color,
    ) -> Light {
        Light::Area(AreaLight::new(
            corner, full_uvec, usteps, full_vvec, vsteps, intensity,
        ))
    }
    // A point light is a 1x1 grid whose only sample is its position; an area
    // light reports its real grid. `lighting` and `intensity_at` iterate these
    // uniformly, so both light kinds flow through the same code.
    pub fn usteps(&self) -> usize {
        match self {
            Light::Point(_) => 1,
            Light::Area(a) => a.usteps,
        }
    }
    pub fn vsteps(&self) -> usize {
        match self {
            Light::Point(_) => 1,
            Light::Area(a) => a.vsteps,
        }
    }
    pub fn samples(&self) -> usize {
        match self {
            Light::Point(_) => 1,
            Light::Area(a) => a.samples,
        }
    }
    pub fn point_on_light(&self, u: usize, v: usize) -> Point {
        match self {
            Light::Point(p) => p.position,
            Light::Area(a) => a.point_on_light(u, v),
        }
    }
}

impl PointLight {
    pub const fn new(position: Point, intensity: Color) -> Self {
        Self {
            position,
            intensity,
        }
    }
}

pub trait LightProperties {
    fn position(&self) -> Point;
    fn intensity(&self) -> Color;
}

impl LightProperties for PointLight {
    fn position(&self) -> Point {
        self.position
    }
    fn intensity(&self) -> Color {
        self.intensity
    }
}

impl LightProperties for Light {
    fn position(&self) -> Point {
        match self {
            Light::Point(light) => light.position,
            Light::Area(light) => light.position,
        }
    }
    fn intensity(&self) -> Color {
        match self {
            Light::Point(light) => light.intensity,
            Light::Area(light) => light.intensity,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_light_has_a_position_and_intensity() {
        let intensity = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        };
        let position = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let light = Light::Point(PointLight {
            position: position,
            intensity: intensity,
        });
        assert_eq!(light.position(), position);
        assert_eq!(light.intensity(), intensity);
    }

    fn white() -> Color {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        }
    }

    #[test]
    fn creating_an_area_light() {
        let corner = Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let v1 = Vector {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        };
        let v2 = Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let light = AreaLight::new(corner, v1, 4, v2, 2, white());
        assert_eq!(light.corner, corner);
        assert_eq!(light.uvec, Vector { x: 0.5, y: 0.0, z: 0.0 });
        assert_eq!(light.usteps, 4);
        assert_eq!(light.vvec, Vector { x: 0.0, y: 0.0, z: 0.5 });
        assert_eq!(light.vsteps, 2);
        assert_eq!(light.samples, 8);
        assert_eq!(light.position, Point { x: 1.0, y: 0.0, z: 0.5 });
    }

    #[test]
    fn finding_a_single_point_on_an_area_light() {
        let light = AreaLight::new(
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Vector {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            4,
            Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            2,
            white(),
        );
        let cases = [
            (0, 0, Point { x: 0.25, y: 0.0, z: 0.25 }),
            (1, 0, Point { x: 0.75, y: 0.0, z: 0.25 }),
            (0, 1, Point { x: 0.25, y: 0.0, z: 0.75 }),
            (2, 0, Point { x: 1.25, y: 0.0, z: 0.25 }),
            (3, 1, Point { x: 1.75, y: 0.0, z: 0.75 }),
        ];
        for (u, v, expected) in cases {
            assert_eq!(light.point_on_light(u, v), expected, "u={u} v={v}");
        }
    }
}
