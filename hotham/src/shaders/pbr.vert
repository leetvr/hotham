// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec4 inTangent;
layout (location = 3) in vec2 inUV;
layout (location = 4) in vec4 inJoint;
layout (location = 5) in vec4 inWeight;

layout (location = 0) out vec4 outGlobalPos;
layout (location = 1) out vec2 outUV;
layout (location = 2) flat out uint outMaterialID;
layout (location = 3) out mat3 outTBN;

out gl_PerVertex {
	vec4 gl_Position;
};

void main() {
	DrawData d = drawDataBuffer.data[gl_InstanceIndex];

	if (d.skinID == NOT_PRESENT) {
		// Mesh has no skin
		outGlobalPos = d.globalFromLocal * vec4(inPos, 1.0);
		vec3 normal = normalize(inNormal);
		vec3 tangent = normalize(inTangent.xyz);

		vec3 globalNormal = normalize(vec3(d.inverseTranspose * vec4(normal, 0.0)));
		vec3 globalTangent = normalize(vec3(d.globalFromLocal * vec4(tangent, 0.0)));
		vec3 globalBiTangent = cross(globalNormal, globalTangent) * inTangent.w;
		outTBN = mat3(globalTangent, globalBiTangent, globalNormal);
	} else {
		// Mesh is skinned
		mat4[MAX_JOINTS] jointMatrices = skinsBuffer.jointMatrices[d.skinID];
		mat4 skinMatrix =
			inWeight.x * jointMatrices[int(inJoint.x)] +
			inWeight.y * jointMatrices[int(inJoint.y)] +
			inWeight.z * jointMatrices[int(inJoint.z)] +
			inWeight.w * jointMatrices[int(inJoint.w)];

		outGlobalPos = d.globalFromLocal * skinMatrix * vec4(inPos, 1.0);

		mat3 m3_skinMatrix = mat3(skinMatrix);
		vec3 normal = normalize(m3_skinMatrix * inNormal);
		vec3 tangent = normalize(m3_skinMatrix * inTangent.xyz);

		vec3 globalNormal = normalize(vec3(d.inverseTranspose * vec4(normal, 0.0)));
		vec3 globalTangent = normalize(vec3(d.globalFromLocal * vec4(tangent, 0.0)));
		vec3 globalBiTangent = cross(globalNormal, globalTangent) * inTangent.w;
		outTBN = mat3(globalTangent, globalBiTangent, globalNormal);
	}

	outUV = inUV;
	outMaterialID = d.materialID;
	gl_Position = sceneData.viewProjection[gl_ViewIndex] * outGlobalPos;
}
