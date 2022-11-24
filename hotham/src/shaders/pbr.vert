// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460
#extension GL_EXT_multiview : enable

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) in uint inJoint;
layout (location = 4) in uint inWeight;

layout (location = 0) out vec4 outGosPos;
layout (location = 1) out vec2 outUV;
layout (location = 2) out vec3 outNormal;

struct DrawData {
    mat4 gosFromLocal;
    mat4 localFromGos;
    uint materialID;
    uint skinID;
};

layout (set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData data[5000];
} drawDataBuffer;

layout (std430, set = 0, binding = 1) readonly buffer SkinsBuffer {
    mat4 jointMatrices[100][64];
} skinsBuffer;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    uint skinID = drawDataBuffer.data[gl_InstanceIndex].skinID;
    mat4 gosFromLocal = drawDataBuffer.data[gl_InstanceIndex].gosFromLocal;
    mat4 localFromGos = drawDataBuffer.data[gl_InstanceIndex].localFromGos;

    if (skinID == NOT_PRESENT) {
        // Mesh has no skin
        outGosPos = gosFromLocal * vec4(inPos, 1.0);
        outNormal = normalize(inNormal * mat3(localFromGos));
    } else {
        // Mesh is skinned
        // Shift and mask to unpack the individual indices and weights.
        // There is no need to divide with the sum of weights because we are using homogenous coordinates.
        mat4 skinMatrix =
            ((inWeight) & 255)       * skinsBuffer.jointMatrices[skinID][(inJoint) & 255] +
            ((inWeight >> 8) & 255)  * skinsBuffer.jointMatrices[skinID][(inJoint >> 8) & 255] +
            ((inWeight >> 16) & 255) * skinsBuffer.jointMatrices[skinID][(inJoint >> 16) & 255] +
            ((inWeight >> 24) & 255) * skinsBuffer.jointMatrices[skinID][(inJoint >> 24) & 255];

        outGosPos = gosFromLocal * skinMatrix * vec4(inPos, 1.0);
        outNormal = normalize(mat3(skinMatrix) * inNormal * mat3(localFromGos));
    }

    outUV = inUV;
    gl_Position = sceneData.viewProjection[gl_ViewIndex] * outGosPos;
}
