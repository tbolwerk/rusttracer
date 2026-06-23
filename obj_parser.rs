// A small subset of the Wavefront OBJ format, following chapter 15 of "The Ray
// Tracer Challenge". Supported records:
//   v  x y z      a vertex (1-indexed, in the order they appear)
//   vn x y z      a vertex normal (1-indexed)
//   f  ...        a face: three or more vertex references, fan-triangulated.
//                 References may be `v`, `v/vt/vn` or `v//vn`; texture indices
//                 are ignored. A face whose references all carry a normal
//                 becomes a smooth triangle, otherwise a flat one.
//   g  name       starts a new named group; faces after it go into that group.
// Anything else (including the gibberish the book throws at it) is counted in
// `ignored` and skipped, so a malformed line never aborts the parse.

use crate::materials::Material;
use crate::shapes::{HasMaterial, Shape};
use crate::tuples::*;
use crate::worlds::World;
use std::cmp::Ordering;

// One named group of triangles. The parser always starts with a default group
// (the empty name); a `g` record opens another. Triangles are stored as ready
// `Shape`s so they can be dropped straight into a `World`.
#[derive(Debug)]
pub struct ObjGroup {
    pub name: String,
    pub triangles: Vec<Shape>,
}

#[derive(Debug)]
pub struct ObjParser {
    pub ignored: usize,
    // 1-indexed to match OBJ's numbering: index 0 is an unused placeholder so
    // that `vertices[1]` is the first vertex.
    pub vertices: Vec<Point>,
    pub normals: Vec<Vector>,
    pub groups: Vec<ObjGroup>,
}

impl ObjParser {
    // The group faces land in before any `g` record is seen.
    pub fn default_group(&self) -> &ObjGroup {
        &self.groups[0]
    }
    // Look up a named group, e.g. one opened by `g FirstGroup`.
    pub fn group(&self, name: &str) -> Option<&ObjGroup> {
        self.groups.iter().find(|g| g.name == name)
    }
    // Pour the parsed geometry into `world` as one parent group with a child
    // group per non-empty OBJ group, and return the parent's arena id. This is
    // the book's `obj_to_group`, adapted to the index-based arena: the caller can
    // transform the returned group and should call `world.compute_bounds()`
    // afterwards so the many triangles get bounding-box culling.
    pub fn to_world(&self, world: &mut World) -> usize {
        let root = world.add_object(Shape::group());
        for group in &self.groups {
            if group.triangles.is_empty() {
                continue;
            }
            let child = world.add_child(root, Shape::group());
            for triangle in &group.triangles {
                world.add_child(child, triangle.clone());
            }
        }
        root
    }

    // Load every triangle (across all OBJ groups) into `world` as a bounding
    // volume hierarchy: a tree of nested groups built by repeatedly splitting the
    // triangles at the median of their longest axis. `to_world` leaves all
    // triangles in one flat group, so a ray that hits the model's box still tests
    // every triangle; this gives the existing per-group culling real structure,
    // turning that linear scan into a roughly logarithmic one. A model with
    // thousands of triangles is only usable in the live viewer this way.
    //
    // `leaf_size` caps how many triangles sit directly in a leaf group, and
    // `material` is applied to every triangle (OBJ files here carry no materials).
    // Returns the root group's arena id; call `world.compute_bounds()` afterwards.
    pub fn to_world_bvh(&self, world: &mut World, leaf_size: usize, material: Material) -> usize {
        build_from_triangles(world, self.all_triangles(), leaf_size, material)
    }

    // Like `to_world_bvh`, but first gives the model smooth (Phong) shading by
    // synthesizing a normal at every vertex. OBJ files like the teapot ship with
    // faces only, so they render faceted; `smoothed` averages the face normals
    // meeting at each vertex and rebuilds every flat triangle as a smooth one,
    // letting the smooth-triangle path interpolate across each face. Averaging
    // rounds off genuinely sharp edges, but for an organic model like the teapot
    // the result reads as the intended curved surface.
    pub fn to_world_bvh_smooth(
        &self,
        world: &mut World,
        leaf_size: usize,
        material: Material,
    ) -> usize {
        build_from_triangles(world, smoothed(self.all_triangles()), leaf_size, material)
    }

    // Every triangle across every OBJ group, flattened into one list.
    fn all_triangles(&self) -> Vec<Shape> {
        self.groups
            .iter()
            .flat_map(|g| g.triangles.iter().cloned())
            .collect()
    }
}

// Paint each triangle with `material`, then pack the lot into a BVH under a fresh
// root group, returning its arena id.
fn build_from_triangles(
    world: &mut World,
    mut triangles: Vec<Shape>,
    leaf_size: usize,
    material: Material,
) -> usize {
    for triangle in &mut triangles {
        triangle.set_material(material.clone());
    }
    let root = world.add_object(Shape::group());
    build_bvh(world, root, triangles, leaf_size.max(1));
    root
}

// Convert flat triangles to smooth ones by giving each vertex a normal equal to
// the (area-weighted) average of the faces meeting there. Vertices are matched
// by position: a shared OBJ vertex parses to identical coordinates, so quantizing
// to a fine grid groups exactly the triangles that touch. A vertex whose face
// normals cancel falls back to that triangle's flat normal. Non-triangle shapes
// pass through unchanged.
fn smoothed(triangles: Vec<Shape>) -> Vec<Shape> {
    use std::collections::HashMap;
    // Quantize a position into an integer key. 1e4 keeps ~4 decimals, far finer
    // than the spacing between distinct vertices but coarse enough to ignore
    // float noise.
    fn key(p: Point) -> (i64, i64, i64) {
        let q = |v: Number| (v * 10_000.0).round() as i64;
        (q(p.x), q(p.y), q(p.z))
    }

    // Accumulate area-weighted face normals at each vertex. `e2.cross(e1)` matches
    // the orientation the flat triangle uses for its own normal, and its length is
    // proportional to the face area, so larger faces pull the average more.
    let mut accum: HashMap<(i64, i64, i64), Vector> = HashMap::new();
    for shape in &triangles {
        if let Shape::Triangle(t) = shape {
            let face = t.e2.cross(t.e1);
            for p in [t.p1, t.p2, t.p3] {
                let entry = accum.entry(key(p)).or_insert(Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                });
                *entry = *entry + face;
            }
        }
    }

    triangles
        .into_iter()
        .map(|shape| match shape {
            Shape::Triangle(t) => {
                let normal_at = |p: Point, flat: Vector| {
                    let summed = accum.get(&key(p)).copied().unwrap_or(flat);
                    if summed.magnitude() < EPSILON {
                        flat
                    } else {
                        summed.normalize()
                    }
                };
                let flat = t.e2.cross(t.e1);
                Shape::smooth_triangle(
                    t.p1,
                    t.p2,
                    t.p3,
                    normal_at(t.p1, flat),
                    normal_at(t.p2, flat),
                    normal_at(t.p3, flat),
                )
            }
            other => other,
        })
        .collect()
}

// The centroid of a triangle shape, used to decide which side of a split it
// falls on. Non-triangle shapes never reach here, so they map to the origin.
fn triangle_centroid(shape: &Shape) -> Point {
    let (a, b, c) = match shape {
        Shape::Triangle(t) => (t.p1, t.p2, t.p3),
        Shape::SmoothTriangle(t) => (t.p1, t.p2, t.p3),
        _ => return Point {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    };
    Point {
        x: (a.x + b.x + c.x) / 3.0,
        y: (a.y + b.y + c.y) / 3.0,
        z: (a.z + b.z + c.z) / 3.0,
    }
}

fn axis_value(p: Point, axis: usize) -> Number {
    match axis {
        0 => p.x,
        1 => p.y,
        _ => p.z,
    }
}

// Recursively partition `triangles` under `parent`. Small slices become a leaf
// group holding the triangles directly; larger ones are split in two at the
// median of whichever axis their centroids spread across most, each half going
// into its own child group. Children are always created after their parent, so
// the arena ids stay monotonic and `World::compute_bounds` (which finalizes in
// reverse id order) sees every child's box before the parent's.
fn build_bvh(world: &mut World, parent: usize, mut triangles: Vec<Shape>, leaf_size: usize) {
    if triangles.len() <= leaf_size {
        for triangle in triangles {
            world.add_child(parent, triangle);
        }
        return;
    }

    // Spread of the centroids on each axis; split the widest one.
    let mut lo = [Number::INFINITY; 3];
    let mut hi = [Number::NEG_INFINITY; 3];
    for triangle in &triangles {
        let c = triangle_centroid(triangle);
        for axis in 0..3 {
            let v = axis_value(c, axis);
            lo[axis] = lo[axis].min(v);
            hi[axis] = hi[axis].max(v);
        }
    }
    let extent = |axis: usize| hi[axis] - lo[axis];
    let axis = if extent(0) >= extent(1) && extent(0) >= extent(2) {
        0
    } else if extent(1) >= extent(2) {
        1
    } else {
        2
    };

    triangles.sort_by(|x, y| {
        axis_value(triangle_centroid(x), axis)
            .partial_cmp(&axis_value(triangle_centroid(y), axis))
            .unwrap_or(Ordering::Equal)
    });

    let mid = triangles.len() / 2;
    let right = triangles.split_off(mid);
    let left_group = world.add_child(parent, Shape::group());
    build_bvh(world, left_group, triangles, leaf_size);
    let right_group = world.add_child(parent, Shape::group());
    build_bvh(world, right_group, right, leaf_size);
}

// Parse a face's vertex reference: `v`, `v/vt/vn` or `v//vn`. Returns the
// 1-based vertex index and, when present, the 1-based normal index. Texture
// indices (the middle field) are accepted but ignored. Returns None if the
// vertex index is missing or unparseable.
fn parse_face_vertex(token: &str) -> Option<(usize, Option<usize>)> {
    let mut fields = token.split('/');
    let vertex = fields.next()?.parse::<usize>().ok()?;
    let _texture = fields.next(); // ignored
    let normal = match fields.next() {
        Some(s) if !s.is_empty() => Some(s.parse::<usize>().ok()?),
        _ => None,
    };
    Some((vertex, normal))
}

// Fan-triangulate one face into triangle shapes. With vertices v0..vn-1 the fan
// is (v0, v1, v2), (v0, v2, v3), ... A triangle whose three references all have
// a normal becomes a smooth triangle. Returns None (so the caller can count the
// line as ignored) if any reference is malformed or out of range.
fn parse_face(specs: &[&str], vertices: &[Point], normals: &[Vector]) -> Option<Vec<Shape>> {
    if specs.len() < 3 {
        return None;
    }
    let refs: Vec<(usize, Option<usize>)> = specs
        .iter()
        .map(|s| parse_face_vertex(s))
        .collect::<Option<_>>()?;

    let mut triangles = Vec::with_capacity(refs.len() - 2);
    for i in 1..refs.len() - 1 {
        let (v1, n1) = refs[0];
        let (v2, n2) = refs[i];
        let (v3, n3) = refs[i + 1];
        let p1 = *vertices.get(v1)?;
        let p2 = *vertices.get(v2)?;
        let p3 = *vertices.get(v3)?;
        match (n1, n2, n3) {
            (Some(a), Some(b), Some(c)) => {
                let na = *normals.get(a)?;
                let nb = *normals.get(b)?;
                let nc = *normals.get(c)?;
                triangles.push(Shape::smooth_triangle(p1, p2, p3, na, nb, nc));
            }
            _ => triangles.push(Shape::triangle(p1, p2, p3)),
        }
    }
    Some(triangles)
}

// Parse `count` whitespace-separated floats, returning None if any field fails
// to parse. Extra fields (such as a vertex's optional w) are ignored.
fn parse_floats(fields: &[&str], count: usize) -> Option<Vec<Number>> {
    if fields.len() < count {
        return None;
    }
    fields[..count].iter().map(|s| s.parse().ok()).collect()
}

pub fn parse_obj(input: &str) -> ObjParser {
    let mut ignored = 0;
    // Index 0 is a placeholder so the rest are 1-indexed like the file.
    let mut vertices = vec![Point {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }];
    let mut normals = vec![Vector {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }];
    let mut groups = vec![ObjGroup {
        name: String::new(),
        triangles: vec![],
    }];
    let mut current = 0; // index into `groups`

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue; // blank lines are structural, not "ignored"
        }
        let mut tokens = trimmed.split_whitespace();
        let keyword = tokens.next();
        let rest: Vec<&str> = tokens.collect();
        match keyword {
            Some("v") => match parse_floats(&rest, 3) {
                Some(f) => vertices.push(Point {
                    x: f[0],
                    y: f[1],
                    z: f[2],
                }),
                None => ignored += 1,
            },
            Some("vn") => match parse_floats(&rest, 3) {
                Some(f) => normals.push(Vector {
                    x: f[0],
                    y: f[1],
                    z: f[2],
                }),
                None => ignored += 1,
            },
            Some("f") => match parse_face(&rest, &vertices, &normals) {
                Some(triangles) => groups[current].triangles.extend(triangles),
                None => ignored += 1,
            },
            Some("g") => {
                groups.push(ObjGroup {
                    name: rest.first().unwrap_or(&"").to_string(),
                    triangles: vec![],
                });
                current = groups.len() - 1;
            }
            _ => ignored += 1,
        }
    }

    ObjParser {
        ignored,
        vertices,
        normals,
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triangles::{SmoothTriangle, Triangle};

    fn as_triangle(s: &Shape) -> &Triangle {
        match s {
            Shape::Triangle(t) => t,
            _ => panic!("expected a flat triangle"),
        }
    }
    fn as_smooth(s: &Shape) -> &SmoothTriangle {
        match s {
            Shape::SmoothTriangle(t) => t,
            _ => panic!("expected a smooth triangle"),
        }
    }

    #[test]
    fn ignoring_unrecognized_lines() {
        let gibberish = "\
There was a young lady named Bright
who traveled much faster than light.
She set out one day
in a relative way,
and came back the previous night.";
        let parser = parse_obj(gibberish);
        assert_eq!(parser.ignored, 5);
    }

    #[test]
    fn vertex_records() {
        let input = "\
v -1 1 0
v -1.0000 0.5000 0.0000
v 1 0 0
v 1 1 0";
        let parser = parse_obj(input);
        assert_eq!(parser.vertices[1], Point { x: -1.0, y: 1.0, z: 0.0 });
        assert_eq!(parser.vertices[2], Point { x: -1.0, y: 0.5, z: 0.0 });
        assert_eq!(parser.vertices[3], Point { x: 1.0, y: 0.0, z: 0.0 });
        assert_eq!(parser.vertices[4], Point { x: 1.0, y: 1.0, z: 0.0 });
    }

    #[test]
    fn parsing_triangle_faces() {
        let input = "\
v -1 1 0
v -1 0 0
v 1 0 0
v 1 1 0

f 1 2 3
f 1 3 4";
        let parser = parse_obj(input);
        let g = parser.default_group();
        let t1 = as_triangle(&g.triangles[0]);
        let t2 = as_triangle(&g.triangles[1]);
        assert_eq!(t1.p1, parser.vertices[1]);
        assert_eq!(t1.p2, parser.vertices[2]);
        assert_eq!(t1.p3, parser.vertices[3]);
        assert_eq!(t2.p1, parser.vertices[1]);
        assert_eq!(t2.p2, parser.vertices[3]);
        assert_eq!(t2.p3, parser.vertices[4]);
    }

    #[test]
    fn triangulating_polygons() {
        let input = "\
v -1 1 0
v -1 0 0
v 1 0 0
v 1 1 0
v 0 2 0

f 1 2 3 4 5";
        let parser = parse_obj(input);
        let g = parser.default_group();
        let t1 = as_triangle(&g.triangles[0]);
        let t2 = as_triangle(&g.triangles[1]);
        let t3 = as_triangle(&g.triangles[2]);
        assert_eq!(t1.p1, parser.vertices[1]);
        assert_eq!(t1.p2, parser.vertices[2]);
        assert_eq!(t1.p3, parser.vertices[3]);
        assert_eq!(t2.p1, parser.vertices[1]);
        assert_eq!(t2.p2, parser.vertices[3]);
        assert_eq!(t2.p3, parser.vertices[4]);
        assert_eq!(t3.p1, parser.vertices[1]);
        assert_eq!(t3.p2, parser.vertices[4]);
        assert_eq!(t3.p3, parser.vertices[5]);
    }

    #[test]
    fn triangles_in_groups() {
        let input = "\
v -1 1 0
v -1 0 0
v 1 0 0
v 1 1 0

g FirstGroup
f 1 2 3
g SecondGroup
f 1 3 4";
        let parser = parse_obj(input);
        let t1 = as_triangle(&parser.group("FirstGroup").unwrap().triangles[0]);
        let t2 = as_triangle(&parser.group("SecondGroup").unwrap().triangles[0]);
        assert_eq!(t1.p1, parser.vertices[1]);
        assert_eq!(t1.p2, parser.vertices[2]);
        assert_eq!(t1.p3, parser.vertices[3]);
        assert_eq!(t2.p1, parser.vertices[1]);
        assert_eq!(t2.p2, parser.vertices[3]);
        assert_eq!(t2.p3, parser.vertices[4]);
    }

    #[test]
    fn converting_an_obj_file_to_a_group() {
        let input = "\
v -1 1 0
v -1 0 0
v 1 0 0
v 1 1 0

g FirstGroup
f 1 2 3
g SecondGroup
f 1 3 4";
        let parser = parse_obj(input);
        let mut w = World::new();
        let root = parser.to_world(&mut w);
        // The root group holds one child group per non-empty OBJ group.
        let children = match &w.objects[root] {
            Shape::Group(g) => g.children.clone(),
            _ => panic!("expected a group"),
        };
        assert_eq!(children.len(), 2);
        for child in &children {
            assert!(matches!(w.objects[*child], Shape::Group(_)));
        }
        // The first child group is FirstGroup, holding triangle (v1, v2, v3).
        let first_tri_id = match &w.objects[children[0]] {
            Shape::Group(g) => g.children[0],
            _ => panic!("expected a group"),
        };
        let t = as_triangle(&w.objects[first_tri_id]);
        assert_eq!(t.p1, parser.vertices[1]);
        assert_eq!(t.p2, parser.vertices[2]);
        assert_eq!(t.p3, parser.vertices[3]);
    }

    #[test]
    fn vertex_normal_records() {
        let input = "\
vn 0 0 1
vn 0.707 0 -0.707
vn 1 2 3";
        let parser = parse_obj(input);
        assert_eq!(parser.normals[1], Vector { x: 0.0, y: 0.0, z: 1.0 });
        assert_eq!(
            parser.normals[2],
            Vector {
                x: 0.707,
                y: 0.0,
                z: -0.707
            }
        );
        assert_eq!(parser.normals[3], Vector { x: 1.0, y: 2.0, z: 3.0 });
    }

    #[test]
    fn faces_with_normals() {
        let input = "\
v 0 1 0
v -1 0 0
v 1 0 0

vn -1 0 0
vn 1 0 0
vn 0 1 0

f 1//3 2//1 3//2
f 1/0/3 2/102/1 3/14/2";
        let parser = parse_obj(input);
        let g = parser.default_group();
        let t1 = as_smooth(&g.triangles[0]);
        let t2 = as_smooth(&g.triangles[1]);
        assert_eq!(t1.p1, parser.vertices[1]);
        assert_eq!(t1.p2, parser.vertices[2]);
        assert_eq!(t1.p3, parser.vertices[3]);
        assert_eq!(t1.n1, parser.normals[3]);
        assert_eq!(t1.n2, parser.normals[1]);
        assert_eq!(t1.n3, parser.normals[2]);
        // The second face uses the same data written with texture indices.
        assert_eq!(t2.p1, t1.p1);
        assert_eq!(t2.p2, t1.p2);
        assert_eq!(t2.p3, t1.p3);
        assert_eq!(t2.n1, t1.n1);
        assert_eq!(t2.n2, t1.n2);
        assert_eq!(t2.n3, t1.n3);
    }
}
