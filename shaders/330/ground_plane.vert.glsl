#version 330 core

in vec3 v_pos;
in vec2 v_tex;
uniform mat4 proj_mat, view_mat, model_mat;
out vec2 tex_coord;


void main() {
    tex_coord = v_tex;
    gl_Position = proj_mat * view_mat * model_mat * vec4 (v_pos, 1.0);
}
