// PBR shader based on Sascha Williems' implementation: 
// https://github.com/SaschaWillems/Vulkan-glTF-PBR 
// Which in turn was based on https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460

#include "common.glsl"

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) in vec4 inJoint0;
layout (location = 4) in vec4 inWeight0;

layout (location = 0) out vec4 outWorldPos;
layout (location = 1) out vec3 outNormal;
layout (location = 2) out vec2 outUV;
layout (location = 3) out uint outMaterialID;

out gl_PerVertex
{
	vec4 gl_Position;
};

void main() 
{
	DrawData d = drawDataBuffer.data[gl_DrawID];
	vec4 localPosition;
	// if (node.jointCount > 0.0) {
	// 	// Mesh is skinned
	// 	mat4 skinMat = 
	// 		inWeight0.x * node.jointMatrix[int(inJoint0.x)] +
	// 		inWeight0.y * node.jointMatrix[int(inJoint0.y)] +
	// 		inWeight0.z * node.jointMatrix[int(inJoint0.z)] +
	// 		inWeight0.w * node.jointMatrix[int(inJoint0.w)];

	// 	locPos = node.matrix * skinMat * vec4(inPos, 1.0);
	// 	outNormal = normalize(transpose(inverse(mat3(node.matrix * skinMat))) * inNormal);
	// } else {
	localPosition = d.transform * vec4(inPos, 1.0);
	// }

	if (length(inNormal) == 0.0) {
		outNormal = inNormal;
	} else {
		outNormal = normalize(mat3(d.inverseTranspose) * inNormal);
	}

	outWorldPos = localPosition;
	outUV = inUV;
	outMaterialID = d.materialID;
	gl_Position = sceneData.viewProjection[gl_ViewIndex] * localPosition;
}