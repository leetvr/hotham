#version 460

#include "../../../../hotham/src/shaders/common.glsl"

layout (location = 0) in vec3 inPos;

layout (location = 0) out vec4 outGosPos;
layout (location = 1) out flat uint outInstanceIndex;

#include "quadric.glsl"

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    QuadricData d = quadricDataBuffer.data[gl_InstanceIndex];
    outInstanceIndex = gl_InstanceIndex;
    outGosPos = d.gosFromLocal * vec4(inPos, 1.0);
    gl_Position = sceneData.viewProjection[gl_ViewIndex] * outGosPos;
}
