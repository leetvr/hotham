#version 450
#extension GL_ARB_separate_shader_objects : enable

mat4 rotation3d(vec3 axis, float angle) {
  axis = normalize(axis);
  float s = sin(angle);
  float c = cos(angle);
  float oc = 1.0 - c;

  return mat4(
		oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,  0.0,
        oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,  0.0,
        oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c,           0.0,
		0.0,                                0.0,                                0.0,                                1.0
    );
}

mat4 scaleVec(vec3 scale) {
    return mat4(
        scale.x,    0,          0,          0,
        0,          scale.y,    0,          0,
        0,          0,          scale.z,    0,
        0,          0,          0,          1
    );
}

mat4 translateVec(vec3 translate) {
    return mat4(
        1,          0,          0,          translate.x,
        0,          1,          0,          translate.y,
        0,          0,          1,          translate.z,
        0,          0,          0,          1
    );
}

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 projection;
    float deltaTime;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 outColor;

void main() {
    gl_Position = ubo.projection * ubo.view * ubo.model * vec4(inPosition, 1.0);
    outColor = inColor;
}