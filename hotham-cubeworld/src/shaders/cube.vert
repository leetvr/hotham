#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(binding = 0) uniform UniformBufferObject {
    mat4 modelView;
    mat4 projection;
    float deltaTime;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 outColor;

void main() {
    gl_Position = ubo.projection * ubo.modelView * vec4(inPosition, 1.0);
    outColor = inColor;
}