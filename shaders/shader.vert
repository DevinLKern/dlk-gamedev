#version 450

layout(std140, set = 0, binding = 0) uniform CameraUBO {
    mat4 model;
    mat4 view;
    mat4 proj;
} mvp;

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 texCoord;

// layout(location = 1) out vec3 vColor;
layout(location = 0) out vec2 vTexCoord;

void main() {
    gl_Position = mvp.proj * mvp.view * mvp.model * vec4(position, 1);
    // vColor = vec3(1.0, 0.1, 0.5);
    vTexCoord = texCoord;
}
