use math::Vector3;


pub struct PointLight {
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub specular_exponent: f32,
    pub position: Vector3,
}

impl PointLight {
    pub fn new(
        ambient: f32, diffuse: f32, specular: f32, specular_exponent: f32,
        position: Vector3) -> PointLight {

        PointLight {
            ambient: ambient,
            diffuse: diffuse,
            specular: specular,
            specular_exponent: specular_exponent,
            position: position,
        }
    }
}
