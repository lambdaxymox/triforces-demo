use std::fs::File;
use std::io::{Seek, SeekFrom, BufRead, BufReader};


///
/// An `ObjMesh` is a model space representation of a 3D geometric figure.
/// You typically generate one from parsing a Wavefront *.obj file into
/// an `ObjMesh`.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ObjMesh {
    pub point_count: usize,
    pub points: Vec<f32>,
    pub tex_coords: Vec<f32>,
    pub normals: Vec<f32>,
}

impl ObjMesh {
    ///
    /// Generate a new mesh object.
    ///
    fn new(points: Vec<f32>, tex_coords: Vec<f32>, normals: Vec<f32>) -> ObjMesh {
        ObjMesh {
            point_count: points.len() / 3,
            points: points,
            tex_coords: tex_coords,
            normals: normals,
        }
    }

    ///
    /// Present the points map as an array slice. This function can be used
    /// to present the internal array buffer to OpenGL or another Graphics
    /// system for rendering.
    ///
    #[inline]
    fn points(&self) -> &[f32] {
        &self.points
    }

    ///
    /// Present the texture map as an array slice. This function can be used
    /// to present the internal array buffer to OpenGL or another Graphics
    /// system for rendering.
    ///
    #[inline]
    fn tex_coords(&self) -> &[f32] {
        &self.tex_coords
    }

    ///
    /// Present the normal vector map as an array slice. This function can be used
    /// to present the internal array buffer to OpenGL or another Graphics
    /// system for rendering.
    ///
    #[inline]
    fn normals(&self) -> &[f32] {
        &self.normals
    }
}

struct UnsortedVertexData {
    vp: Vec<f32>,
    vt: Vec<f32>,
    vn: Vec<f32>,
}

struct SortedVertexData {
    points: Vec<f32>,
    tex_coords: Vec<f32>,
    normals: Vec<f32>,
}

fn skip_spaces(bytes: &[u8]) -> usize {
    let mut index = 0;
    while index < bytes.len() - 1 {
        if bytes[index] == b' ' || bytes[index] == b'\\' {
            index += 1;
        } else {
            break;
        }
    }

    index
}

fn count_vertices<T: BufRead + Seek>(reader: &mut T) -> (usize, usize, usize, usize) {
    let mut unsorted_vp_count = 0;
    let mut unsorted_vt_count = 0;
    let mut unsorted_vn_count = 0;
    let mut face_count = 0;

    for line in reader.lines().map(|st| st.unwrap()) {
        let bytes = line.as_bytes();
        let i = skip_spaces(bytes);
        match bytes[i] {
            b'v' => match bytes[i + 1] {
                b' ' => unsorted_vp_count += 1,
                b't' => unsorted_vt_count += 1,
                b'n' => unsorted_vn_count += 1,
                _ => {},
            }
            b'f' => {
                face_count += 1;
            }
            _ => {}
        }
    }

    reader.seek(SeekFrom::Start(0)).unwrap();

    (unsorted_vp_count, unsorted_vt_count, unsorted_vn_count, face_count)
}

fn is_valid_vtn_triple(
    tuple: &(Option<u32>, Option<u32>, Option<u32>,
             Option<u32>, Option<u32>, Option<u32>,
             Option<u32>, Option<u32>, Option<u32>)) -> bool {

    tuple.0.is_some() && tuple.1.is_some() && tuple.2.is_some() &&
        tuple.3.is_some() && tuple.4.is_some() && tuple.5.is_some() &&
        tuple.6.is_some() && tuple.7.is_some() && tuple.8.is_some()
}

fn parse_vtn(
    line: &str,
    unsorted_vtn: &mut UnsortedVertexData, sorted_vtn: &mut SortedVertexData) -> Result<(), String> {

    // First, try parsing the line as though there are texture vertices.
    let tuple = scan_fmt!(
        line, "f {}/{}/{} {}/{}/{} {}/{}/{}", u32, u32, u32, u32, u32, u32, u32, u32, u32
    );

    if !is_valid_vtn_triple(&tuple) {
        return Err(format!("Invalid mesh face declaration: {}", line));
    }

    let (vp0, vt0, vn0, vp1, vt1, vn1, vp2, vt2, vn2) = tuple;
    let vp = [vp0.unwrap(), vp1.unwrap(), vp2.unwrap()];
    let vt = [vt0.unwrap(), vt1.unwrap(), vt2.unwrap()];
    let vn = [vn0.unwrap(), vn1.unwrap(), vn2.unwrap()];

    // Start reading points into a buffer. order is -1 because
    // obj starts from 1, not 0.
    // NB: assuming all indices are valid
    for j in 0..3 {
        if vp[j] - 1 >= unsorted_vtn.vp.len() as u32 {
            return Err(format!("ERROR: invalid vertex position index in face"));
        }
        if vt[j] - 1 >= unsorted_vtn.vt.len() as u32 {
            return Err(format!("ERROR: invalid texture coord index {} in face.", vt[j]));
        }
        if vn[j] - 1 >= unsorted_vtn.vn.len() as u32 {
            return Err(format!("ERROR: invalid vertex normal index in face"));
        }
    }

    for j in 0..3 {
        sorted_vtn.points.push(unsorted_vtn.vp[((vp[j] - 1) * 3) as usize]);
        sorted_vtn.points.push(unsorted_vtn.vp[((vp[j] - 1) * 3 + 1) as usize]);
        sorted_vtn.points.push(unsorted_vtn.vp[((vp[j] - 1) * 3 + 2) as usize]);

        sorted_vtn.tex_coords.push(unsorted_vtn.vt[((vt[j] - 1) * 2) as usize]);
        sorted_vtn.tex_coords.push(unsorted_vtn.vt[((vt[j] - 1) * 2 + 1) as usize]);

        sorted_vtn.normals.push(unsorted_vtn.vn[((vn[j] - 1) * 3) as usize]);
        sorted_vtn.normals.push(unsorted_vtn.vn[((vn[j] - 1) * 3 + 1) as usize]);
        sorted_vtn.normals.push(unsorted_vtn.vn[((vn[j] - 1) * 3 + 2) as usize]);
    }

    Ok(())
}

fn is_valid_vn_triple(
    tuple: &(Option<u32>, Option<u32>, Option<u32>,
             Option<u32>, Option<u32>, Option<u32>)) -> bool {

    tuple.0.is_some() && tuple.1.is_some() && tuple.2.is_some() &&
        tuple.3.is_some() && tuple.4.is_some() && tuple.5.is_some()
}

fn parse_vn(
    line: &str,
    unsorted_vtn: &mut UnsortedVertexData, sorted_vtn: &mut SortedVertexData) -> Result<(), String> {

    // First, try parsing the line as though there are texture vertices.
    let tuple = scan_fmt!(
        line, "f {}//{} {}//{} {}//{}", u32, u32, u32, u32, u32, u32
    );

    if !is_valid_vn_triple(&tuple) {
        return Err(format!("Invalid mesh face declaration: \"{}\"", line));
    }

    let (vp0, vn0, vp1, vn1, vp2, vn2) = tuple;
    let vp = [vp0.unwrap(), vp1.unwrap(), vp2.unwrap()];
    let vn = [vn0.unwrap(), vn1.unwrap(), vn2.unwrap()];

    // Start reading points into a buffer. order is -1 because
    // obj starts from 1, not 0.
    // NB: assuming all indices are valid
    for j in 0..3 {
        if vp[j] - 1 >= unsorted_vtn.vp.len() as u32 {
            return Err(format!("ERROR: invalid vertex position index in face"));
        }
        if vn[j] - 1 >= unsorted_vtn.vn.len() as u32 {
            return Err(format!("ERROR: invalid vertex normal index in face"));
        }
    }

    for j in 0..3 {
        sorted_vtn.points.push(unsorted_vtn.vp[((vp[j] - 1) * 3) as usize]);
        sorted_vtn.points.push(unsorted_vtn.vp[((vp[j] - 1) * 3 + 1) as usize]);
        sorted_vtn.points.push(unsorted_vtn.vp[((vp[j] - 1) * 3 + 2) as usize]);

        sorted_vtn.normals.push(unsorted_vtn.vn[((vn[j] - 1) * 3) as usize]);
        sorted_vtn.normals.push(unsorted_vtn.vn[((vn[j] - 1) * 3 + 1) as usize]);
        sorted_vtn.normals.push(unsorted_vtn.vn[((vn[j] - 1) * 3 + 2) as usize]);
    }

    Ok(())
}

pub fn load_obj_mesh<T: BufRead + Seek>(reader: &mut T) -> Result<ObjMesh, String> {
    // First, we count the number of vertices, texture vertices, normal vectors, and faces
    // in the file so we know how much memory to allocate.
    let (unsorted_vp_count, unsorted_vt_count, unsorted_vn_count, _) = count_vertices(reader);

    let mut unsorted_vtn = UnsortedVertexData {
        vp: vec![0.0; 3 * unsorted_vp_count],
        vt: vec![0.0; 2 * unsorted_vt_count],
        vn: vec![0.0; 3 * unsorted_vn_count],
    };

    let mut sorted_vtn = SortedVertexData {
        points: vec![],
        tex_coords: vec![],
        normals: vec![]
    };

    let mut current_unsorted_vp = 0;
    let mut current_unsorted_vt = 0;
    let mut current_unsorted_vn = 0;

    for line in reader.lines().map(|st| st.unwrap()) {
        let bytes = line.as_bytes();
        let i = skip_spaces(bytes);
        if bytes[i] == b'v' {
            // Vertex line.
            if bytes[i + 1] == b' ' {
                // Vertex point.
                let (x, y, z) = scan_fmt!(&line, "v {} {} {}", f32, f32, f32);
                unsorted_vtn.vp[current_unsorted_vp * 3]     = x.unwrap();
                unsorted_vtn.vp[current_unsorted_vp * 3 + 1] = y.unwrap();
                unsorted_vtn.vp[current_unsorted_vp * 3 + 2] = z.unwrap();
                current_unsorted_vp += 1;
            } else if bytes[i + 1] == b't' {
                // Vertex texture coordinate.
                let (s, t) = scan_fmt!(&line, "vt {} {}", f32, f32);
                unsorted_vtn.vt[current_unsorted_vt * 2]     = s.unwrap();
                unsorted_vtn.vt[current_unsorted_vt * 2 + 1] = t.unwrap();
                current_unsorted_vt += 1;
            } else if bytes[i + 1] == b'n' {
                // Vertex normal coordinate.
                let (x, y, z) = scan_fmt!(&line, "vn {} {} {}", f32, f32, f32);
                unsorted_vtn.vn[current_unsorted_vn * 3]     = x.unwrap();
                unsorted_vtn.vn[current_unsorted_vn * 3 + 1] = y.unwrap();
                unsorted_vtn.vn[current_unsorted_vn * 3 + 2] = z.unwrap();
                current_unsorted_vn += 1;
            }
        } else if bytes[i] == b'f' {
            // Face line.
            // work out if using quads instead of triangles and print a warning
            let mut slash_count = 0;
            for j in i..bytes.len() {
                if bytes[j] == b'/' {
                    slash_count += 1;
                }
            }
            if slash_count != 6 {
                return Err(format!(
                    "ERROR: file contains quads or does not match v vp/vt/vn layout -
                     make sure exported mesh is triangulated and contains vertex points,
                     texture coordinates, and normals"
                ));
            }

            let result = parse_vtn(&line, &mut unsorted_vtn, &mut sorted_vtn);
            if result.is_err() {
                let result = parse_vn(&line, &mut unsorted_vtn, &mut sorted_vtn);
                if result.is_err() {
                    return Err(format!(
                        "ERROR: This file contains a face element that is neither
                         a vp/vt/vn index or a vp//vn index. Got line \"{}\"",
                        line
                    ));
                }
            }
        }
    }

    Ok(ObjMesh::new(sorted_vtn.points, sorted_vtn.tex_coords, sorted_vtn.normals))
}

pub fn load_obj_file(file_name: &str) -> Result<ObjMesh, String> {
    let file = match File::open(file_name) {
        Ok(handle) => handle,
        Err(_) => {
            return Err(format!("ERROR: file not found: {}", file_name));
        }
    };

    let mut reader = BufReader::new(file);
    load_obj_mesh(&mut reader)
}

mod parser_tests {
    use super::ObjMesh;
    use std::io::{BufReader, Cursor};

    struct Test {
        obj_file: String,
        obj_mesh: ObjMesh,
        vp_count: usize,
        vt_count: usize,
        vn_count: usize,
        face_count: usize,

    }

    fn test() -> Test {
        let obj_file = String::from(r"        \
            o object1                         \
            g cube                            \
            v  0.0  0.0  0.0                  \
            v  0.0  0.0  1.0                  \
            v  0.0  1.0  0.0                  \
            v  0.0  1.0  1.0                  \
            v  1.0  0.0  0.0                  \
            v  1.0  0.0  1.0                  \
            v  1.0  1.0  0.0                  \
            v  1.0  1.0  1.0                  \
                                              \
            vn  0.0  0.0  1.0                 \
            vn  0.0  0.0 -1.0                 \
            vn  0.0  1.0  0.0                 \
            vn  0.0 -1.0  0.0                 \
            vn  1.0  0.0  0.0                 \
            vn -1.0  0.0  0.0                 \
                                              \
            f  1//2  7//2  5//2               \
            f  1//2  3//2  7//2               \
            f  1//6  4//6  3//6               \
            f  1//6  2//6  4//6               \
            f  3//3  8//3  7//3               \
            f  3//3  4//3  8//3               \
            f  5//5  7//5  8//5               \
            f  5//5  8//5  6//5               \
            f  1//4  5//4  6//4               \
            f  1//4  6//4  2//4               \
            f  2//1  6//1  8//1               \
            f  2//1  8//1  4//1               \
        ");
        let point_count = 36;
        let points = vec![
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0,
            0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
            0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0,
        ];
        let tex_coords = vec![];
        let normals = vec![
            0.0,  0.0, -1.0,  0.0,  0.0, -1.0,  0.0,  0.0, -1.0,
            0.0,  0.0, -1.0,  0.0,  0.0, -1.0,  0.0,  0.0, -1.0,
            -1.0,  0.0,  0.0, -1.0,  0.0,  0.0, -1.0,  0.0,  0.0,
            -1.0,  0.0,  0.0, -1.0,  0.0,  0.0, -1.0,  0.0,  0.0,
            0.0,  1.0,  0.0,  0.0,  1.0,  0.0,  0.0,  1.0,  0.0,
            0.0,  1.0,  0.0,  0.0,  1.0,  0.0,  0.0,  1.0,  0.0,
            1.0,  0.0,  0.0,  1.0,  0.0,  0.0,  1.0,  0.0,  0.0,
            1.0,  0.0,  0.0,  1.0,  0.0,  0.0,  1.0,  0.0,  0.0,
            0.0, -1.0,  0.0,  0.0, -1.0,  0.0,  0.0, -1.0,  0.0,
            0.0, -1.0,  0.0,  0.0, -1.0,  0.0,  0.0, -1.0,  0.0,
            0.0,  0.0,  1.0,  0.0,  0.0,  1.0,  0.0,  0.0,  1.0,
            0.0,  0.0,  1.0,  0.0,  0.0,  1.0,  0.0,  0.0,  1.0,
        ];

        let obj_mesh = ObjMesh {
            point_count: point_count,
            points: points,
            tex_coords: tex_coords,
            normals: normals,
        };

        Test {
            obj_file: obj_file,
            obj_mesh: obj_mesh,
            vp_count: 8,
            vt_count: 0,
            vn_count: 6,
            face_count: 12,
        }
    }

    #[test]
    fn test_count_vertices() {
        let test = test();
        let mut reader = BufReader::new(Cursor::new(test.obj_file.as_bytes()));
        let (unsorted_vp_count,
            unsorted_vt_count,
            unsorted_vn_count,
            face_count) = super::count_vertices(&mut reader);

        assert_eq!(unsorted_vp_count, test.vp_count);
        assert_eq!(unsorted_vt_count, test.vt_count);
        assert_eq!(unsorted_vn_count, test.vn_count);
        assert_eq!(face_count, test.face_count);
    }

    #[test]
    fn test_parse_obj_mesh_elementwise() {
        let test = test();
        let mut reader = BufReader::new(Cursor::new(test.obj_file.as_bytes()));
        let result = super::load_obj_mesh(&mut reader).unwrap();
        let expected = test.obj_mesh;

        assert_eq!(result.point_count, expected.point_count);
        assert_eq!(result.points, expected.points);
        assert_eq!(result.tex_coords, expected.tex_coords);
        assert_eq!(result.normals, expected.normals);
    }

    #[test]
    fn test_parse_obj_mesh() {
        let test = test();
        let mut reader = BufReader::new(Cursor::new(test.obj_file.as_bytes()));
        let result = super::load_obj_mesh(&mut reader).unwrap();
        let expected = test.obj_mesh;

        assert_eq!(result, expected);
    }
}

