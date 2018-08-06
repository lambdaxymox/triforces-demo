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
    vec4 K = texture (tex, tex_coord);
    vec3 n_eye = normalize (normal_eye);
    vec3 Ia = vec3 (light.La * K);

    vec3 Id = vec3(0.0, 0.0, 0.0);

    vec3 Is = vec3(0.0, 0.0, 0.0);
    frag_color = vec4 (Ia + Id + Is, 1.0);
}
