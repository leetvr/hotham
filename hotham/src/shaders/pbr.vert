// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec4 inTangent;
layout (location = 3) in vec2 inUV;
layout (location = 4) in uint inJoint;
layout (location = 5) in uint inWeight;

layout (location = 0) out vec4 outGlobalPos;
layout (location = 1) out vec2 outUV;
layout (location = 2) flat out uint outMaterialID;
layout (location = 3) out vec3 outNormal;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    DrawData d = drawDataBuffer.data[gl_InstanceIndex];

    if (d.skinID == NOT_PRESENT) {
        // Mesh has no skin
        outGlobalPos = d.globalFromLocal * vec4(inPos, 1.0);
        outNormal = normalize(inNormal * mat3(d.localFromGlobal));
    } else {
        // Mesh is skinned
        // Shift and mask to unpack the individual indices and weights.
        // There is no need to divide with the sum of weights because we are using homogenous coordinates.
        mat4 skinMatrix =
            ((inWeight) & 255)       * skinsBuffer.jointMatrices[d.skinID][(inJoint) & 255] +
            ((inWeight >> 8) & 255)  * skinsBuffer.jointMatrices[d.skinID][(inJoint >> 8) & 255] +
            ((inWeight >> 16) & 255) * skinsBuffer.jointMatrices[d.skinID][(inJoint >> 16) & 255] +
            ((inWeight >> 24) & 255) * skinsBuffer.jointMatrices[d.skinID][(inJoint >> 24) & 255];

        outGlobalPos = d.globalFromLocal * skinMatrix * vec4(inPos, 1.0);
        outNormal = normalize(mat3(skinMatrix) * inNormal * mat3(d.localFromGlobal));
    }

    outUV = inUV;
    outMaterialID = d.materialID;
    gl_Position = sceneData.viewProjection[gl_ViewIndex] * outGlobalPos;
}
