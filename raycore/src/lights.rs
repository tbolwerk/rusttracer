use crate::tuples::*;

// A single flat, tagged light struct so the same layout works on the CPU and on
// rust-gpu/SPIR-V (no data-carrying enums). `kind` selects the behavior:
//   0 = point light: a single emitter at `position`.
//   1 = area light: a `usteps` x `vsteps` grid of cells spanning the rectangle
//       `corner + full_uvec + full_vvec`. Shadow rays are cast to a point in each
//       cell and averaged, so an occluder casts a soft penumbra rather than a hard
//       edge. `uvec`/`vvec` are the per-cell step vectors and `position` is the
//       rectangle's center (used where a single point is needed).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub kind: u32, // 0 = point, 1 = area
    pub position: Point,
    pub intensity: Color,
    pub corner: Point, // area only
    pub uvec: Vector,  // area only, per-cell step (full_uvec / usteps)
    pub vvec: Vector,  // area only, per-cell step (full_vvec / vsteps)
    pub usteps: usize,
    pub vsteps: usize,
    pub samples: usize,
}

impl Light {
    pub const fn point_light(position: Point, intensity: Color) -> Light {
        let zero = Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        Light {
            kind: 0,
            position,
            intensity,
            corner: Point {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            uvec: zero,
            vvec: zero,
            usteps: 1,
            vsteps: 1,
            samples: 1,
        }
    }
    pub fn area_light(
        corner: Point,
        full_uvec: Vector,
        usteps: usize,
        full_vvec: Vector,
        vsteps: usize,
        intensity: Color,
    ) -> Light {
        Light {
            kind: 1,
            position: corner + full_uvec * 0.5 + full_vvec * 0.5,
            intensity,
            corner,
            uvec: full_uvec * (1.0 / usteps as Number),
            vvec: full_vvec * (1.0 / vsteps as Number),
            usteps,
            vsteps,
            samples: usteps * vsteps,
        }
    }
    // A point light is a 1x1 grid whose only sample is its position; an area
    // light reports its real grid. `lighting` and `intensity_at` iterate these
    // uniformly, so both light kinds flow through the same code.
    pub fn usteps(&self) -> usize {
        self.usteps
    }
    pub fn vsteps(&self) -> usize {
        self.vsteps
    }
    pub fn samples(&self) -> usize {
        self.samples
    }
    pub fn position(&self) -> Point {
        self.position
    }
    pub fn intensity(&self) -> Color {
        self.intensity
    }
    // The center of cell (u, v). For a point light this is just its position.
    // Sampling cell centers (the +0.5 offset) gives a fixed, deterministic
    // pattern; the book optionally jitters within each cell for smoother
    // penumbras, which is omitted here so renders stay reproducible across the
    // parallel renderer.
    pub fn point_on_light(&self, u: usize, v: usize) -> Point {
        if self.kind == 0 {
            self.position
        } else {
            self.corner + self.uvec * (u as Number + 0.5) + self.vvec * (v as Number + 0.5)
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
        let light = Light::point_light(position, intensity);
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
        let light = Light::area_light(corner, v1, 4, v2, 2, white());
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
        let light = Light::area_light(
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
