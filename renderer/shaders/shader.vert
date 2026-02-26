#version 450

const uint MESH_FLAG_TEXTURED_BIT = 1;

// set 0 is for objects that are updated every frame
layout(std140, set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 proj;
} camera;

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

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tex_coord;
layout(location = 2) in vec3 normal;

// layout(location = 1) out vec3 vColor;
layout(location = 0) out vec2 v_tex_coord;

void main() {
    float a = world_light.ambient;
    gl_Position = camera.proj * camera.view * mesh.model * vec4(position, 1);
    v_tex_coord = tex_coord;
}
