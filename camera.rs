use crate::canvas::Canvas;
use crate::colors::Pixel;
use crate::matrices::*;
use crate::rays::*;
use crate::transformations::rotation_y;
use crate::transformations::translation;
use crate::transformations::PI;
use crate::tuples::*;
use crate::worlds::*;
use rayon::prelude::*;
use std::ops::Div;

pub struct Camera<const HSIZE: usize, const VSIZE: usize> {
    field_of_view: f32,
    transform: Matrix<4, 4>,
    inverse_transform: Option<Matrix<4, 4>>,
    pixel_size: f32,
    half_width: f32,
    half_height: f32,
}
impl<const HSIZE: usize, const VSIZE: usize> Camera<HSIZE, VSIZE> {
    pub fn new(field_of_view: f32) -> Self {
        let half_view = field_of_view.div(2.0).tan();
        let aspect = HSIZE as f32 / VSIZE as f32;

        let (half_width, half_height) = if aspect >= 1.0 {
            (half_view, half_view / aspect)
        } else {
            (half_view * aspect, half_view)
        };

        let pixel_size = (half_width * 2.0) / HSIZE as f32;

        Self {
            field_of_view: field_of_view,
            transform: Matrix::identity(),
            inverse_transform: None,
            pixel_size,
            half_width: half_width,
            half_height: half_height,
        }
    }
    pub fn ray_for_pixel(&self, px: usize, py: usize) -> Ray {
        let xoffset = (px as f32 + 0.5) * self.pixel_size;
        let yoffset = (py as f32 + 0.5) * self.pixel_size;
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
    pub fn set_transform(&mut self, transform: Matrix<4, 4>) -> () {
        self.transform = transform;
        self.inverse_transform = inverse(&transform);
    }
    pub fn render(&self, world: World) -> Canvas<HSIZE, VSIZE> {
        let mut image: Canvas<HSIZE, VSIZE> = Canvas::new(255);
        for y in 0..VSIZE {
            for x in 0..HSIZE {
                let ray = self.ray_for_pixel(x, y);
                let color = world.color_at(&ray);
                image.write_pixel(color, y, x);
            }
        }
        image
    }
    pub fn render_par(&self, world: World) -> Canvas<HSIZE, VSIZE> {
        let mut image: Canvas<HSIZE, VSIZE> = Canvas::new(255);
        image
            .pixels
            .par_rows_mut()
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..HSIZE {
                    let ray = self.ray_for_pixel(x, y);
                    let color = world.color_at(&ray);

                    row[x] = Pixel::clamp(0, 255, color);
                }
            });

        image
    }
}

mod tests {
    use super::*;

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
        assert_eq!(c.pixel_size, 0.01);
    }
    #[test]
    fn the_pixel_size_for_a_vertical_canvas() {
        const HSIZE: usize = 125;
        const VSIZE: usize = 200;
        let field_of_view = PI / 2.0;
        let c: Camera<HSIZE, VSIZE> = Camera::new(field_of_view);
        assert_eq!(c.pixel_size, 0.01);
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
                x: 2.0_f32.sqrt() / 2.0,
                y: 0.0,
                z: -2.0_f32.sqrt() / 2.0,
            }
        )
    }
}
