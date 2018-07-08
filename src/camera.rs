use math::{Vector3, Vector4, Matrix4, Quaternion};


pub struct Camera {
    pub near: f32,
    pub far: f32,
    pub fov: f32,
    pub aspect: f32,
    pub proj_mat: Matrix4,

    pub cam_speed: f32,
    pub cam_yaw_speed: f32,
    pub cam_pos: Vector3,
    pub fwd: Vector4,
    pub rgt: Vector4,
    pub up: Vector4,

    pub trans_mat_inv: Matrix4,
    pub axis: Quaternion,
    pub rot_mat_inv: Matrix4,
    pub view_mat: Matrix4,
}

impl Camera {
    pub fn new(
        near: f32, far: f32, fov: f32, aspect: f32, 
        cam_speed: f32, cam_yaw_speed: f32, cam_pos: Vector3,
        fwd: Vector4, rgt: Vector4, up: Vector4, axis: Quaternion) -> Camera {

        let proj_mat = Matrix4::perspective(fov, aspect, near, far);
        let trans_mat_inv = Matrix4::identity().translate(&cam_pos);
        let rot_mat_inv = axis.to_mat4();
        let view_mat = rot_mat_inv.inverse() * trans_mat_inv.inverse();

        Camera {
            near: near,
            far: far,
            fov: fov,
            aspect: aspect,
            proj_mat: proj_mat,

            cam_speed: cam_speed,
            cam_yaw_speed: cam_yaw_speed,
            cam_pos: cam_pos,
            fwd: fwd,
            rgt: rgt,
            up: up,

            trans_mat_inv: trans_mat_inv,
            axis: axis,
            rot_mat_inv: rot_mat_inv,
            view_mat: view_mat,
        }
    }
}

