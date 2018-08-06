use math::Vector3;


pub struct PointLight {
    pub ambient: Vector3,
    pub diffuse: Vector3,
    pub specular: Vector3,
    pub specular_exponent: f32,
    pub position: Vector3,
}

impl PointLight {
    pub fn new(
        ambient: Vector3, diffuse: Vector3, specular: Vector3,
        specular_exponent: f32,
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
