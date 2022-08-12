// PBR shader based on a combination of Sascha Willems' implementation:
// https://github.com/SaschaWillems/Vulkan-glTF-PBR
//
// and the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460
#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

// Textures
layout(set = 0, binding = 4) uniform sampler2D textures[];
layout(set = 0, binding = 5) uniform samplerCube cubeTextures[];

#include "pbr.glsl"

#define ERROR_MAGENTA vec4(1., 0., 1., 1.)

// Inputs
layout (location = 0) in vec3 inGlobalPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) flat in uint inMaterialID;
layout (location = 4) in mat3 inTBN;

// Outputs
layout(location = 0) out vec4 outColor;

// Get normal, tangent and bitangent vectors.
vec3 getNormal(uint normalTextureID)
{
    vec3 n, t, b, ng;

    // Trivial TBN computation, present as vertex attribute.
    // Normalize eigenvectors as matrix is linearly interpolated.
    t = normalize(inTBN[0]);
    b = normalize(inTBN[1]);
    ng = normalize(inTBN[2]);

	vec3 ntex;
	ntex.xy = texture(textures[normalTextureID], inUV).ga * 2.0 - 1.0;
	ntex.z = sqrt(1 - dot(ntex.xy, ntex.xy));

    return normalize(mat3(t, b, ng) * ntex);
}

void main() {
	// Start by setting the output color to a familiar "error" magenta.
	outColor = ERROR_MAGENTA;

	// Retrieve the material from the buffer.
	Material material = materialBuffer.materials[inMaterialID];

	// Determine the base color
	vec4 baseColor;

	if (material.baseColorTextureID == NOT_PRESENT) {
		baseColor = material.baseColorFactor;
	} else {
		baseColor = texture(textures[material.baseColorTextureID], inUV) * material.baseColorFactor;
	}

	// Handle transparency
	if (material.alphaMask == 1.0f) {
		if (baseColor.a < material.alphaMaskCutoff) {
			// TODO: Apparently Adreno GPUs don't like discarding.
			discard;
		}
	}

	// Choose the correct workflow for this material
	if (material.workflow == PBR_WORKFLOW_METALLIC_ROUGHNESS) {
		// Get the normal
		vec3 n = (material.normalTextureID == NOT_PRESENT) ? normalize(inNormal) : getNormal(material.normalTextureID);

		outColor.rgb = getPBRMetallicRoughnessColor(material, baseColor, inGlobalPos, n, inUV);
	} else if (material.workflow == PBR_WORKFLOW_UNLIT) {
		outColor = baseColor;
	}

	// Finally, tonemap the color.
	outColor = tonemap(outColor);

	// Debugging
	// Shader inputs debug visualization
	// "none", "Base Color Texture", "Normal Texture", "Occlusion Texture", "Emissive Texture", "Metallic (?)", "Roughness (?)"
	if (sceneData.params.z > 0.0) {
		int index = int(sceneData.params.z);
		switch (index) {
			case 1:
				outColor.rgba = baseColor;
				break;
			case 2:
				vec3 n = (material.normalTextureID == NOT_PRESENT) ? normalize(inNormal) : getNormal(material.normalTextureID);
				outColor.rgb = n * 0.5 + 0.5;
				break;
			case 3:
				outColor.rgb = (material.occlusionTextureID == NOT_PRESENT) ? vec3(0.0f) : texture(textures[material.occlusionTextureID], inUV).ggg;
				break;
			case 4:
				outColor.rgb = (material.emissiveTextureID == NOT_PRESENT) ?  vec3(0.0f) : texture(textures[material.emissiveTextureID], inUV).rgb;
				break;
			case 5:
				outColor.rgb = (material.physicalDescriptorTextureID == NOT_PRESENT) ? vec3(0.0) : texture(textures[material.physicalDescriptorTextureID], inUV).ggg;
				break;
			case 6:
				outColor.rgb = (material.physicalDescriptorTextureID == NOT_PRESENT) ? vec3(0.0) : texture(textures[material.physicalDescriptorTextureID], inUV).aaa;
				break;
		}
		outColor = outColor;
	}
}
