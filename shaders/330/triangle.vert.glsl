#version 330 core

in vec3 v_pos;
in vec2 v_tex;
in vec3 v_norm;

uniform mat4 proj_mat;
uniform mat4 view_mat;
uniform mat4 model_mat;

out vec3 position_eye;
out vec2 tex_coord;
out vec3 normal_eye;


void main() {
    position_eye = vec3 (view_mat * model_mat * vec4 (v_pos, 1.0));
    tex_coord = v_tex;
    normal_eye = vec3 (view_mat * model_mat * vec4 (v_norm, 0.0));
    gl_Position = proj_mat * vec4 (position_eye, 1.0);
}
