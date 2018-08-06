#version 420 core

in vec3 position_eye;
in vec2 tex_coord;
in vec3 normal_eye;

uniform mat4 view_mat;
uniform mat4 model_mat;
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
    vec3 K = vec3 (texture (tex, tex_coord));
    vec3 norm_eye = normalize (normal_eye);
    vec3 Ia = light.La * K;

    vec3 light_pos_eye = vec3 (view_mat * vec4 (light.pos_wor, 1.0));
    vec3 dist_to_light_eye = light_pos_eye - position_eye;
    vec3 dir_to_light_eye = normalize (dist_to_light_eye);
    float dot_diffuse = dot (dir_to_light_eye, norm_eye);
    dot_diffuse = max (dot_diffuse, 0.0);
    vec3 Id = light.Ld * K * dot_diffuse;

    vec3 Is = vec3(0.0, 0.0, 0.0);
    frag_color = vec4 (Ia + Id + Is, 1.0);
}
