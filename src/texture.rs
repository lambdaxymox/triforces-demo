use stb_image::image;


#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }
}

impl Default for Rgba {
    #[inline]
    fn default() -> Rgba {
        Rgba::new(0, 0, 0, 255)
    }
}

pub struct TexImage2D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub data: Vec<Rgba>,
}

impl TexImage2D {
    pub fn new(width: u32, height: u32) -> TexImage2D {
        TexImage2D {
            width: width,
            height: height,
            depth: 4,
            data: vec![Rgba::default(); (width * height) as usize],
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        &self.data[0].r
    }
}

impl<'a> From<&'a image::Image<u8>> for TexImage2D {
    fn from(image: &'a image::Image<u8>) -> TexImage2D {
        let mut data = vec![];
        for chunk in image.data.chunks(4) {
            data.push(Rgba::new(chunk[0], chunk[1], chunk[2], chunk[3]));
        }

        TexImage2D {
            width: image.width as u32,
            height: image.height as u32,
            depth: image.depth as u32,
            data: data,
        }
    }
}

