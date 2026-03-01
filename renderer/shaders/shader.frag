#version 450

const uint MESH_FLAG_TEXTURED_BIT = 1;

// set 1 is for objects that are update every object
layout(std140, set = 1, binding = 0) uniform MeshUBO {
    mat4 model;
    vec4 base_color;
    uint flags;
} mesh;

layout (set = 2, binding = 1) uniform sampler2D tex_sampler;

// layout (location = 0) in vec3 vColor;
layout (location = 0) in vec2 v_tex_coord;

layout (location = 0) out vec4 f_color;


void main() {
    if (mesh.flags == MESH_FLAG_TEXTURED_BIT) {
        f_color = texture(tex_sampler, v_tex_coord);
    } else {
        f_color = mesh.base_color;
    }
}
