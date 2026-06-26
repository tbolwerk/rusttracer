// The geometry of texture mapping (the "Texture Mapping" bonus chapter): 2D UV
// patterns, the functions that map a 3D point on a shape to (u, v) coordinates,
// and cube-face mapping. The `Pattern::Texture`/`Pattern::Cube` variants in
// `patterns.rs` glue these to the rest of the renderer.

use crate::transformations::PI;
use crate::tuples::*;

const fn black() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    }
}

// A pattern defined in 2D (u, v) space, independent of any 3D shape.
// Flat tagged struct for rust-gpu/SPIR-V compatibility:
//   kind 0 = checkers (uses width/height/a/b)
//   kind 1 = align_check (uses main/ul/ur/bl/br)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvFace {
    pub kind: u32,
    pub width: Number,
    pub height: Number,
    pub a: Color,
    pub b: Color,
    pub main: Color,
    pub ul: Color,
    pub ur: Color,
    pub bl: Color,
    pub br: Color,
}

impl UvFace {
    // A checkerboard of `width` x `height` cells across the 0..1 UV square.
    pub const fn checkers(width: Number, height: Number, a: Color, b: Color) -> Self {
        UvFace {
            kind: 0,
            width,
            height,
            a,
            b,
            main: black(),
            ul: black(),
            ur: black(),
            bl: black(),
            br: black(),
        }
    }
    // A center color with a distinct color in each corner, used to verify that a
    // cube face is oriented correctly.
    pub fn align_check(main: Color, ul: Color, ur: Color, bl: Color, br: Color) -> Self {
        UvFace {
            kind: 1,
            width: 0.0,
            height: 0.0,
            a: black(),
            b: black(),
            main,
            ul,
            ur,
            bl,
            br,
        }
    }
    pub fn uv_pattern_at(&self, u: Number, v: Number) -> Color {
        match self.kind {
            0 => {
                let u2 = (u * self.width).floor();
                let v2 = (v * self.height).floor();
                if (u2 + v2).rem_euclid(2.0) == 0.0 {
                    self.a
                } else {
                    self.b
                }
            }
            _ => {
                // Corners get their own color; everything else is `main`.
                if v > 0.8 {
                    if u < 0.2 {
                        return self.ul;
                    }
                    if u > 0.8 {
                        return self.ur;
                    }
                } else if v < 0.2 {
                    if u < 0.2 {
                        return self.bl;
                    }
                    if u > 0.8 {
                        return self.br;
                    }
                }
                self.main
            }
        }
    }
}

// How a 3D point in pattern space is reduced to (u, v), as a u32 tag:
//   0 = spherical, 1 = planar, 2 = cylindrical.
pub const MAPPING_SPHERICAL: u32 = 0;
pub const MAPPING_PLANAR: u32 = 1;
pub const MAPPING_CYLINDRICAL: u32 = 2;

pub fn uv_map(p: Point, mapping: u32) -> (Number, Number) {
    match mapping {
        MAPPING_PLANAR => planar_map(p),
        MAPPING_CYLINDRICAL => cylindrical_map(p),
        _ => spherical_map(p),
    }
}

// Wrap a point on a unit sphere to (u, v): u from the angle around +y, v from the
// angle down from +y.
pub fn spherical_map(p: Point) -> (Number, Number) {
    let theta = p.x.atan2(p.z);
    let radius = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
    let phi = (p.y / radius).acos();
    let raw_u = theta / (2.0 * PI);
    let u = 1.0 - (raw_u + 0.5);
    let v = 1.0 - phi / PI;
    (u, v)
}

// Project onto the xz-plane, tiling every unit square.
pub fn planar_map(p: Point) -> (Number, Number) {
    (p.x.rem_euclid(1.0), p.z.rem_euclid(1.0))
}

// Wrap around a unit cylinder: u from the angle around +y, v from height.
pub fn cylindrical_map(p: Point) -> (Number, Number) {
    let theta = p.x.atan2(p.z);
    let raw_u = theta / (2.0 * PI);
    let u = 1.0 - (raw_u + 0.5);
    let v = p.y.rem_euclid(1.0);
    (u, v)
}

// The six faces of a cube, picked by which coordinate of a point is largest.
// repr(u32) so the discriminant isn't u8 (which rust-gpu needs Int8 for).
#[repr(u32)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum CubeFace {
    Left,
    Right,
    Front,
    Back,
    Up,
    Down,
}

pub fn face_from_point(p: Point) -> CubeFace {
    let coord = p.x.abs().max(p.y.abs()).max(p.z.abs());
    if coord == p.x {
        CubeFace::Right
    } else if coord == -p.x {
        CubeFace::Left
    } else if coord == p.y {
        CubeFace::Up
    } else if coord == -p.y {
        CubeFace::Down
    } else if coord == p.z {
        CubeFace::Front
    } else {
        CubeFace::Back
    }
}

// Map a point known to lie on the given cube face to (u, v) within that face.
pub fn cube_uv(face: CubeFace, p: Point) -> (Number, Number) {
    match face {
        CubeFace::Front => (((p.x + 1.0).rem_euclid(2.0)) / 2.0, ((p.y + 1.0).rem_euclid(2.0)) / 2.0),
        CubeFace::Back => (((1.0 - p.x).rem_euclid(2.0)) / 2.0, ((p.y + 1.0).rem_euclid(2.0)) / 2.0),
        CubeFace::Left => (((p.z + 1.0).rem_euclid(2.0)) / 2.0, ((p.y + 1.0).rem_euclid(2.0)) / 2.0),
        CubeFace::Right => (((1.0 - p.z).rem_euclid(2.0)) / 2.0, ((p.y + 1.0).rem_euclid(2.0)) / 2.0),
        CubeFace::Up => (((p.x + 1.0).rem_euclid(2.0)) / 2.0, ((1.0 - p.z).rem_euclid(2.0)) / 2.0),
        CubeFace::Down => (((p.x + 1.0).rem_euclid(2.0)) / 2.0, ((p.z + 1.0).rem_euclid(2.0)) / 2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn black() -> Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }
    }
    fn white() -> Color {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        }
    }

    #[test]
    fn checker_pattern_in_2d() {
        let checkers = UvFace::checkers(2.0, 2.0, black(), white());
        let cases = [
            (0.0, 0.0, black()),
            (0.5, 0.0, white()),
            (0.0, 0.5, white()),
            (0.5, 0.5, black()),
            (1.0, 1.0, black()),
        ];
        for (u, v, expected) in cases {
            assert_eq!(checkers.uv_pattern_at(u, v), expected, "u={u} v={v}");
        }
    }

    #[test]
    fn using_a_spherical_mapping_on_a_3d_point() {
        let s = sqrt(2.0) / 2.0;
        let cases = [
            (Point { x: 0.0, y: 0.0, z: -1.0 }, 0.0, 0.5),
            (Point { x: 1.0, y: 0.0, z: 0.0 }, 0.25, 0.5),
            (Point { x: 0.0, y: 0.0, z: 1.0 }, 0.5, 0.5),
            (Point { x: -1.0, y: 0.0, z: 0.0 }, 0.75, 0.5),
            (Point { x: 0.0, y: 1.0, z: 0.0 }, 0.5, 1.0),
            (Point { x: 0.0, y: -1.0, z: 0.0 }, 0.5, 0.0),
            (Point { x: s, y: s, z: 0.0 }, 0.25, 0.75),
        ];
        for (p, eu, ev) in cases {
            let (u, v) = spherical_map(p);
            assert_almost_eq!(u, eu);
            assert_almost_eq!(v, ev);
        }
    }

    #[test]
    fn using_a_planar_mapping_on_a_3d_point() {
        let cases = [
            (Point { x: 0.25, y: 0.0, z: 0.5 }, 0.25, 0.5),
            (Point { x: 0.25, y: 0.0, z: -0.25 }, 0.25, 0.75),
            (Point { x: 0.25, y: 0.5, z: -0.25 }, 0.25, 0.75),
            (Point { x: 1.25, y: 0.0, z: 0.5 }, 0.25, 0.5),
            (Point { x: 0.25, y: 0.0, z: -1.75 }, 0.25, 0.25),
            (Point { x: 1.0, y: 0.0, z: -1.0 }, 0.0, 0.0),
            (Point { x: 0.0, y: 0.0, z: 0.0 }, 0.0, 0.0),
        ];
        for (p, eu, ev) in cases {
            let (u, v) = planar_map(p);
            assert_almost_eq!(u, eu);
            assert_almost_eq!(v, ev);
        }
    }

    #[test]
    fn using_a_cylindrical_mapping_on_a_3d_point() {
        let s = 0.70711;
        let cases = [
            (Point { x: 0.0, y: 0.0, z: -1.0 }, 0.0, 0.0),
            (Point { x: 0.0, y: 0.5, z: -1.0 }, 0.0, 0.5),
            (Point { x: 0.0, y: 1.0, z: -1.0 }, 0.0, 0.0),
            (Point { x: s, y: 0.5, z: -s }, 0.125, 0.5),
            (Point { x: 1.0, y: 0.5, z: 0.0 }, 0.25, 0.5),
            (Point { x: s, y: 0.5, z: s }, 0.375, 0.5),
            (Point { x: 0.0, y: -0.25, z: 1.0 }, 0.5, 0.75),
            (Point { x: -s, y: 0.5, z: s }, 0.625, 0.5),
            (Point { x: -1.0, y: 1.25, z: 0.0 }, 0.75, 0.25),
            (Point { x: -s, y: 0.5, z: -s }, 0.875, 0.5),
        ];
        for (p, eu, ev) in cases {
            let (u, v) = cylindrical_map(p);
            assert_almost_eq!(u, eu, 1e-4);
            assert_almost_eq!(v, ev, 1e-4);
        }
    }

    #[test]
    fn layout_of_the_align_check_pattern() {
        let main = Color { r: 1.0, g: 1.0, b: 1.0 };
        let ul = Color { r: 1.0, g: 0.0, b: 0.0 };
        let ur = Color { r: 1.0, g: 1.0, b: 0.0 };
        let bl = Color { r: 0.0, g: 1.0, b: 0.0 };
        let br = Color { r: 0.0, g: 1.0, b: 1.0 };
        let pattern = UvFace::align_check(main, ul, ur, bl, br);
        let cases = [
            (0.5, 0.5, main),
            (0.1, 0.9, ul),
            (0.9, 0.9, ur),
            (0.1, 0.1, bl),
            (0.9, 0.1, br),
        ];
        for (u, v, expected) in cases {
            assert_eq!(pattern.uv_pattern_at(u, v), expected, "u={u} v={v}");
        }
    }

    #[test]
    fn identifying_the_face_of_a_cube_from_a_point() {
        let cases = [
            (Point { x: -1.0, y: 0.5, z: -0.25 }, CubeFace::Left),
            (Point { x: 1.1, y: -0.75, z: 0.8 }, CubeFace::Right),
            (Point { x: 0.1, y: 0.6, z: 0.9 }, CubeFace::Front),
            (Point { x: -0.7, y: 0.0, z: -2.0 }, CubeFace::Back),
            (Point { x: 0.5, y: 1.0, z: 0.9 }, CubeFace::Up),
            (Point { x: -0.2, y: -1.3, z: 1.1 }, CubeFace::Down),
        ];
        for (p, expected) in cases {
            assert_eq!(face_from_point(p), expected, "p={p:?}");
        }
    }

    #[test]
    fn uv_mapping_the_front_face_of_a_cube() {
        let cases = [
            (Point { x: -0.5, y: 0.5, z: 1.0 }, 0.25, 0.75),
            (Point { x: 0.5, y: -0.5, z: 1.0 }, 0.75, 0.25),
        ];
        for (p, eu, ev) in cases {
            let (u, v) = cube_uv(CubeFace::Front, p);
            assert_almost_eq!(u, eu);
            assert_almost_eq!(v, ev);
        }
    }

    #[test]
    fn uv_mapping_the_remaining_cube_faces() {
        // (face, point, u, v) drawn from the book's per-face tests.
        let cases = [
            (CubeFace::Back, Point { x: 0.5, y: 0.5, z: -1.0 }, 0.25, 0.75),
            (CubeFace::Back, Point { x: -0.5, y: -0.5, z: -1.0 }, 0.75, 0.25),
            (CubeFace::Left, Point { x: -1.0, y: 0.5, z: -0.5 }, 0.25, 0.75),
            (CubeFace::Left, Point { x: -1.0, y: -0.5, z: 0.5 }, 0.75, 0.25),
            (CubeFace::Right, Point { x: 1.0, y: 0.5, z: 0.5 }, 0.25, 0.75),
            (CubeFace::Right, Point { x: 1.0, y: -0.5, z: -0.5 }, 0.75, 0.25),
            (CubeFace::Up, Point { x: -0.5, y: 1.0, z: -0.5 }, 0.25, 0.75),
            (CubeFace::Up, Point { x: 0.5, y: 1.0, z: 0.5 }, 0.75, 0.25),
            (CubeFace::Down, Point { x: -0.5, y: -1.0, z: 0.5 }, 0.25, 0.75),
            (CubeFace::Down, Point { x: 0.5, y: -1.0, z: -0.5 }, 0.75, 0.25),
        ];
        for (face, p, eu, ev) in cases {
            let (u, v) = cube_uv(face, p);
            assert_almost_eq!(u, eu);
            assert_almost_eq!(v, ev);
        }
    }
}
