#version 450

const uint MESH_FLAG_TEXTURED_BIT = 1;

// set 1 is for objects that are update every object
layout(std140, set = 1, binding = 0) uniform MeshUBO {
    mat4 model;
    vec4 base_color;
    uint flags;
} mesh;

// set 2 is for objects that are updated irregularly
layout(std140, set = 2, binding = 0) uniform GlobalLightUBO {
    vec3 direction;
    vec4 color;
    float ambient;
} world_light;

layout (set = 2, binding = 1) uniform sampler2D tex_sampler;

// layout (location = 0) in vec3 vColor;
layout (location = 0) in vec2 v_tex_coord;
layout (location = 1) in vec3 v_normal;

layout (location = 0) out vec4 f_color;


void main() {
    // This only works when the scale is uniform. Right now it is.
    vec3 normal_world_space = normalize(mat3(mesh.model) * v_normal);
    float light_intensity = world_light.ambient + max(0.0, dot(normal_world_space, -world_light.direction));

    if (mesh.flags == MESH_FLAG_TEXTURED_BIT) {
        f_color = texture(tex_sampler, v_tex_coord) * light_intensity;
    } else {
        f_color = mesh.base_color * light_intensity;
    }
}
