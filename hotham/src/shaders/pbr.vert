// PBR shader based on Sascha Willems' implementation:
// https://github.com/SaschaWillems/Vulkan-glTF-PBR
// Which in turn was based on https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) in vec4 inJoint;
layout (location = 4) in vec4 inWeight;

layout (location = 0) out vec4 outWorldPos;
layout (location = 1) out vec3 outNormal;
layout (location = 2) out vec2 outUV;
layout (location = 3) flat out uint outMaterialID;

out gl_PerVertex
{
	vec4 gl_Position;
};

void main()
{
	DrawData d = drawDataBuffer.data[gl_DrawID];

	if (d.skinID == NO_SKIN) {
		// Mesh has no skin
		outWorldPos = d.transform * vec4(inPos, 1.0);
		if (length(inNormal) == 0.0) {
			outNormal = inNormal;
		} else {
			outNormal = normalize(mat3(d.inverseTranspose) * inNormal);
		}
	} else {
		// Mesh is skinned
		mat4[MAX_JOINTS] jointMatrices = skinsBuffer.jointMatrices[d.skinID];
		mat4 skinMatrix =
			inWeight.x * jointMatrices[int(inJoint.x)] +
			inWeight.y * jointMatrices[int(inJoint.y)] +
			inWeight.z * jointMatrices[int(inJoint.z)] +
			inWeight.w * jointMatrices[int(inJoint.w)];

		outWorldPos = d.transform * skinMatrix * vec4(inPos, 1.0);
		// TODO we should probably be using the inverse transpose here instead?
		outNormal = normalize(mat3(d.inverseTranspose) * mat3(skinMatrix) * inNormal);
	}

	outUV = inUV;
	outMaterialID = d.materialID;
	gl_Position = sceneData.viewProjection[gl_ViewIndex] * outWorldPos;
}
