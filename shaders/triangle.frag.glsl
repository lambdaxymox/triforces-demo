#version 420 core

in vec3 position_eye;
in vec2 tex_coord;
in vec3 normal_eye;

uniform sampler2D tex;

uniform PointLight {
    float La;
    float Ld;
    float Ls;
    float p;
    vec3 pos_wor;
} light;

out vec4 frag_color;


void main() {
    frag_color = texture (tex, tex_coord);
}
