use crate::canvas::Canvas;
use crate::colors::Pixel;
use crate::matrices::*;
use crate::rays::*;
#[cfg(test)]
use crate::transformations::{rotation_y, translation, PI};
use crate::tuples::*;
use crate::worlds::*;
use rayon::prelude::*;
use std::ops::Div;
pub struct Camera<const HSIZE: usize, const VSIZE: usize> {
    field_of_view: Number,
    transform: Matrix<4, 4>,
    inverse_transform: Option<Matrix<4, 4>>,
    pixel_size: Number,
    half_width: Number,
    half_height: Number,
    // Focal blur (the "Focal Blur" bonus chapter). `aperture` is the lens radius:
    // 0.0 is a pinhole (perfectly sharp, the default). With a positive aperture,
    // each pixel averages `samples` rays whose origins are jittered across the
    // lens and which all aim at the same point on the focal plane `focal_distance`
    // away, so only objects at that distance stay sharp.
    aperture: Number,
    focal_distance: Number,
    samples: usize,
}
const MAX_REFLECTION_DEPTH: usize = 5;
impl<const HSIZE: usize, const VSIZE: usize> Camera<HSIZE, VSIZE> {
    pub fn new(field_of_view: Number) -> Self {
        let half_view = field_of_view.div(2.0).tan();
        let aspect = HSIZE as Number / VSIZE as Number;

        let (half_width, half_height) = if aspect >= 1.0 {
            (half_view, half_view / aspect)
        } else {
            (half_view * aspect, half_view)
        };

        let pixel_size = (half_width * 2.0) / HSIZE as Number;

        Self {
            field_of_view: field_of_view,
            transform: Matrix::identity(),
            inverse_transform: None,
            pixel_size,
            half_width: half_width,
            half_height: half_height,
            aperture: 0.0,
            focal_distance: 1.0,
            samples: 1,
        }
    }
    // Enable depth of field: `aperture` is the lens radius (world units), objects
    // at `focal_distance` stay sharp, and `samples` rays per pixel are averaged.
    pub fn set_focal_blur(&mut self, aperture: Number, focal_distance: Number, samples: usize) {
        self.aperture = aperture;
        self.focal_distance = focal_distance.max(EPSILON);
        self.samples = samples.max(1);
    }
    pub fn ray_for_pixel(&self, px: usize, py: usize) -> Ray {
        let xoffset = (px as Number + 0.5) * self.pixel_size;
        let yoffset = (py as Number + 0.5) * self.pixel_size;
        let world_x = self.half_width - xoffset;
        let world_y = self.half_height - yoffset;
        let mut pixel = Point {
            x: world_x,
            y: world_y,
            z: -1.0,
        };
        let mut origin = Point::default();
        match self.inverse_transform {
            None => (),
            Some(inverse_transform) => {
                pixel = inverse_transform * pixel;
                origin = inverse_transform * origin;
            }
        }
        let direction = (pixel - origin.clone()).normalize();
        Ray { origin, direction }
    }
    // A ray through pixel (px, py) originating at lens offset (lens_u, lens_v),
    // each in [-0.5, 0.5]. The ray aims at the pixel's point on the focal plane,
    // so all lens samples for a pixel converge there. With aperture 0 the lens
    // offset has no effect and this reduces to `ray_for_pixel`.
    fn ray_for_pixel_lens(&self, px: usize, py: usize, lens_u: Number, lens_v: Number) -> Ray {
        let xoffset = (px as Number + 0.5) * self.pixel_size;
        let yoffset = (py as Number + 0.5) * self.pixel_size;
        let world_x = self.half_width - xoffset;
        let world_y = self.half_height - yoffset;
        // The point on the focal plane along the central ray through this pixel.
        let mut focus = Point {
            x: world_x * self.focal_distance,
            y: world_y * self.focal_distance,
            z: -self.focal_distance,
        };
        // The ray leaves a jittered point on the lens (z = 0 in camera space).
        let mut origin = Point {
            x: lens_u * self.aperture,
            y: lens_v * self.aperture,
            z: 0.0,
        };
        if let Some(inverse_transform) = self.inverse_transform {
            focus = inverse_transform * focus;
            origin = inverse_transform * origin;
        }
        let direction = (focus - origin).normalize();
        Ray { origin, direction }
    }
    // The averaged color for one pixel. A pinhole camera (aperture 0, 1 sample)
    // casts the single central ray; with focal blur enabled it averages `samples`
    // lens-jittered rays. The jitter is a deterministic hash of (px, py, sample)
    // so it needs no shared RNG state and stays reproducible under the parallel
    // renderer.
    fn color_for_pixel(&self, world: &World, px: usize, py: usize, depth: usize) -> Pixel {
        if self.samples <= 1 && self.aperture == 0.0 {
            let ray = self.ray_for_pixel(px, py);
            return Pixel::clamp(0, 255, world.color_at(&ray, depth));
        }
        let mut sum = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        };
        for s in 0..self.samples {
            let (lens_u, lens_v) = lens_jitter(px, py, s);
            let ray = self.ray_for_pixel_lens(px, py, lens_u, lens_v);
            sum = sum + world.color_at(&ray, depth);
        }
        Pixel::clamp(0, 255, sum * (1.0 / self.samples as Number))
    }
    pub fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse_transform = inverse(&transform);
    }
    // Flatten this camera into the GPU-uploadable `Cam` (pinhole; focal blur is
    // host-only). `max_depth` is the reflection/refraction bounce budget.
    #[cfg(feature = "gpu")]
    pub fn to_cam(&self, max_depth: u32) -> raycore::render::Cam {
        raycore::render::Cam {
            inverse_transform: self.inverse_transform.unwrap_or(Matrix::identity()),
            pixel_size: self.pixel_size,
            half_width: self.half_width,
            half_height: self.half_height,
            hsize: HSIZE as u32,
            vsize: VSIZE as u32,
            max_depth,
        }
    }
    pub fn render(&self, world: World) -> Canvas<VSIZE, HSIZE> {
        let mut image: Canvas<VSIZE, HSIZE> = Canvas::new(255);
        for y in 0..VSIZE {
            for x in 0..HSIZE {
                image.set(self.color_for_pixel(&world, x, y, MAX_REFLECTION_DEPTH), y, x);
            }
        }
        image
    }
    pub fn render_par(&self, world: World) -> Canvas<VSIZE, HSIZE> {
        let mut image: Canvas<VSIZE, HSIZE> = Canvas::new(255);
        image
            .pixels
            .par_rows_mut()
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..HSIZE {
                    row[x] = self.color_for_pixel(&world, x, y, MAX_REFLECTION_DEPTH);
                }
            });
        image
    }
    // Like `render_par`, but borrows the world (so it can be re-rendered every
    // frame without cloning) and takes an explicit recursion depth, so the live
    // flythrough can trade reflection bounces for frame rate.
    pub fn render_live(&self, world: &World, depth: usize) -> Canvas<VSIZE, HSIZE> {
        let mut image: Canvas<VSIZE, HSIZE> = Canvas::new(255);
        image
            .pixels
            .par_rows_mut()
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..HSIZE {
                    row[x] = self.color_for_pixel(world, x, y, depth);
                }
            });
        image
    }
    // Render only rows [y0, y1) directly into the ARGB framebuffer `dst` (a full
    // HSIZE*VSIZE buffer). This lets the viewer build a full-resolution frame in
    // small stripes across several iterations, polling input between them so a
    // slow frame never blocks interaction. Rows within the band still render in
    // parallel.
    pub fn render_live_rows(
        &self,
        world: &World,
        depth: usize,
        y0: usize,
        y1: usize,
        dst: &mut [u32],
    ) {
        let y1 = y1.min(VSIZE);
        if y0 >= y1 {
            return;
        }
        // Parallelize over pixels, not rows: a refinement stripe can be only a few
        // rows tall, and one-task-per-row would leave most cores idle. Rayon splits
        // this contiguous slice across all threads regardless of stripe height.
        dst[y0 * HSIZE..y1 * HSIZE]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, px)| {
                let y = y0 + i / HSIZE;
                let x = i % HSIZE;
                let p = self.color_for_pixel(world, x, y, depth);
                *px = (p.r as u32) << 16 | (p.g as u32) << 8 | p.b as u32;
            });
    }
    // The ARGB color of a single pixel. The viewport's interlaced refinement traces
    // a sparse, growing set of pixels and uses this to color each one through the
    // camera's normal pipeline.
    pub fn pixel_argb(&self, world: &World, px: usize, py: usize, depth: usize) -> u32 {
        let p = self.color_for_pixel(world, px, py, depth);
        (p.r as u32) << 16 | (p.g as u32) << 8 | p.b as u32
    }
}

// A deterministic jitter for lens sampling: hash (px, py, sample) into two values
// in [-0.5, 0.5]. Being a pure function of its inputs, it gives every pixel a
// different but reproducible spread of lens offsets with no shared RNG, which the
// parallel renderer needs.
fn lens_jitter(px: usize, py: usize, sample: usize) -> (Number, Number) {
    fn hash(mut h: u64) -> u64 {
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;
        h
    }
    let base = (px as u64).wrapping_mul(73856093)
        ^ (py as u64).wrapping_mul(19349663)
        ^ (sample as u64).wrapping_mul(83492791);
    let a = hash(base);
    let b = hash(base ^ 0x9e3779b97f4a7c15);
    // Top 53 bits -> [0, 1), then shift to [-0.5, 0.5).
    let to_unit = |x: u64| (x >> 11) as Number / ((1u64 << 53) as Number);
    (to_unit(a) - 0.5, to_unit(b) - 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worlds::World;

    #[test]
    fn render_live_rows_matches_a_full_render() {
        // Striped rendering must produce exactly the same pixels as one full pass,
        // since the viewer assembles the still frame from stripes.
        let mut c: Camera<20, 12> = Camera::new(PI / 2.0);
        c.set_transform(translation(0.0, 0.0, -5.0));
        let world = World::default();
        let full = c.render_live(&world, 2).to_argb();
        let mut banded = vec![0u32; 20 * 12];
        let mut y = 0;
        while y < 12 {
            let y1 = (y + 5).min(12);
            c.render_live_rows(&world, 2, y, y1, &mut banded);
            y = y1;
        }
        assert_eq!(full, banded);
    }

    #[test]
    fn constructing_a_camera() {
        const HSIZE: usize = 160;
        const VSIZE: usize = 120;
        let field_of_view = PI / 2.0;
        let c: Camera<HSIZE, VSIZE> = Camera::new(field_of_view);
        assert_eq!(c.field_of_view, field_of_view);
        assert_eq!(c.transform, Matrix::identity());
    }
    #[test]
    fn the_pixel_size_for_a_horizontal_canvas() {
        const HSIZE: usize = 200;
        const VSIZE: usize = 125;
        let field_of_view = PI / 2.0;
        let c: Camera<HSIZE, VSIZE> = Camera::new(field_of_view);
        assert_almost_eq!(c.pixel_size, 0.01);
    }
    #[test]
    fn the_pixel_size_for_a_vertical_canvas() {
        const HSIZE: usize = 125;
        const VSIZE: usize = 200;
        let field_of_view = PI / 2.0;
        let c: Camera<HSIZE, VSIZE> = Camera::new(field_of_view);
        assert_almost_eq!(c.pixel_size, 0.01);
    }
    #[test]
    fn constructing_a_ray_through_the_center_of_the_canvas() {
        let c: Camera<201, 101> = Camera::new(PI / 2.0);
        let r = c.ray_for_pixel(100, 50);
        assert_eq!(
            r.origin,
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        );
        assert_eq!(
            r.direction,
            Vector {
                x: 0.0,
                y: 0.0,
                z: -1.0
            }
        );
    }
    #[test]
    fn constructing_a_ray_through_the_corner_of_the_canvas() {
        let c: Camera<201, 101> = Camera::new(PI / 2.0);
        let r = c.ray_for_pixel(0, 0);
        assert_eq!(
            r.origin,
            Point {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        );
        assert_eq!(
            r.direction,
            Vector {
                x: 0.66519,
                y: 0.33259,
                z: -0.66851
            }
        )
    }
    #[test]
    fn constructing_a_ray_when_the_camera_is_transformed() {
        let mut c: Camera<201, 101> = Camera::new(PI / 2.0);
        c.set_transform(rotation_y(PI / 4.0) * translation(0.0, -2.0, 5.0));

        let r = c.ray_for_pixel(100, 50);

        assert_eq!(
            r.origin,
            Point {
                x: 0.0,
                y: 2.0,
                z: -5.0
            }
        );
        assert_eq!(
            r.direction,
            Vector {
                x: sqrt(2.0) / 2.0,
                y: 0.0,
                z: -sqrt(2.0) / 2.0,
            }
        )
    }
}
