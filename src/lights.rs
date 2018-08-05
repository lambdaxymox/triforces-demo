use math::Vector3;


pub struct PointLight {
    pub specular: f32,
    pub diffuse: f32,
    pub ambient: f32,
    pub position: Vector3,
}

impl PointLight {
    pub fn new(specular: f32, diffuse: f32, ambient: f32, position: Vector3) -> PointLight {
        PointLight {
            specular: specular,
            diffuse: diffuse,
            ambient: ambient,
            position: position,
        }
    }
}
