// PBR shader based on a combination of Sascha Willems' implementation:
// https://github.com/SaschaWillems/Vulkan-glTF-PBR
// 
// and the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460
#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

const float epsilon = 1e-6;
#define DEFAULT_EXPOSURE 4.5
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS 10
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA vec4(1., 0., 1., 1.)

// Inputs
layout (location = 0) in vec3 inGlobalPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) flat in uint inMaterialID;

// Textures
layout(set = 0, binding = 4) uniform sampler2D textures[];
layout(set = 0, binding = 5) uniform samplerCube cubeTextures[];

// Outputs
layout(location = 0) out vec4 outColor;

// Encapsulate the various inputs used by the various functions in the shading equation
// We store values in this struct to simplify the integration of alternative implementations
// of the shading terms, outlined in the Readme.MD Appendix of Sascha's implementation.
struct MaterialInfo
{
	vec3 f0;                      // full reflectance color (normal incidence angle)
	vec3 f90;                     // reflectance color at grazing angle
	vec3 diffuseColor;            // color contribution from diffuse lighting
	vec3 specularColor;           // color contribution from specular lighting
	float alphaRoughness;         // roughness mapped to a more linear change in the roughness (proposed by [2])
	float perceptualRoughness;    // roughness value, as authored by the model creator (input to shader)
	float NdotV;                  // cos angle between normal and view direction
};

const float PBR_WORKFLOW_METALLIC_ROUGHNESS = 0.0;
const float PBR_WORKFLOW_SPECULAR_GLOSSINESS = 1.0f;
const float PBR_WORKFLOW_UNLIT = 2.0f;

// Anything less than 2% is physically impossible and is instead considered to be shadowing. Compare to "Real-Time-Rendering" 4th editon on page 325.
const vec3 f90 = vec3(1.0);

vec3 Uncharted2Tonemap(vec3 color)
{
	float A = 0.15;
	float B = 0.50;
	float C = 0.10;
	float D = 0.20;
	float E = 0.02;
	float F = 0.30;
	float W = 11.2;
	return ((color*(A*color+C*B)+D*E)/(color*(A*color+B)+D*F))-E/F;
}

vec4 tonemap(vec4 color)
{
	vec3 outcol = Uncharted2Tonemap(color.rgb * DEFAULT_EXPOSURE);
	outcol = outcol * (1.0f / Uncharted2Tonemap(vec3(11.2f)));
	return vec4(outcol, 1.0);
}

// Find the normal for this fragment, pulling either from a predefined normal map
// or from the interpolated mesh normal and tangent attributes.
//
// TODO: We currently use "Normal Mapping Without Precomputed Tangents" as outlined in
// http://www.thetenthplanet.de/archives/1180
//
// This has some potential correctness issues, in addition to being somewhat expensive. The solution
// is to switch to mikktspace tangents: https://github.com/leetvr/hotham/issues/324 
vec3 getNormal(uint normalTextureID)
{
	// We swizzle our normals to save on texture reads: https://github.com/ARM-software/astc-encoder/blob/main/Docs/Encoding.md#encoding-normal-maps
	vec3 tangentNormal;
	tangentNormal.xy = texture(textures[normalTextureID], inUV).ga * 2.0 - 1.0;
	tangentNormal.z = sqrt(1 - dot(tangentNormal.xy, tangentNormal.xy));

	vec3 q1 = dFdx(inGlobalPos);
	vec3 q2 = dFdy(inGlobalPos);
	vec2 st1 = dFdx(inUV);
	vec2 st2 = dFdy(inUV);

	vec3 N = normalize(inNormal);
	vec3 T = normalize(q1 * st2.t - q2 * st1.t);
	vec3 B = -normalize(cross(N, T));
	mat3 TBN = mat3(T, B, N);

	return normalize(TBN * tangentNormal);
}

// Calculation of the lighting contribution from an optional Image Based Light source.
vec3 getIBLContribution(MaterialInfo materialInfo, vec3 n, vec3 reflection)
{
	// Flip the y axis of the reflection vector; otherwise our cubemap will be upside down.
	// TODO: Fix this by pre-flipping the cubemaps.
	reflection.y *= -1.;
	float lod = materialInfo.perceptualRoughness * float(DEFAULT_CUBE_MIPMAP_LEVELS -1);

	// retrieve a scale and bias to F0. See [1], Figure 3
	vec2 brdfSamplePoint = clamp(vec2(materialInfo.NdotV, 1.0 - materialInfo.perceptualRoughness), vec2(0.0, 0.0), vec2(1.0, 1.0));
	vec3 brdf = texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint).rgb;

	// Get diffuse and specular light values
	vec3 diffuseLight = texture(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], n).rgb;
	vec3 specularLight = textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod).rgb;

	// Multiply them by their respective inputs to get their final colors.
	vec3 diffuse = diffuseLight * materialInfo.diffuseColor;
	vec3 specular = specularLight * (materialInfo.specularColor * brdf.x + brdf.y);

	return diffuse + specular;
}

vec3 getLightContribution(MaterialInfo materialInfo, vec3 n, vec3 v, Light light) {
	// Get a vector between this point and the light.
	vec3 pointToLight;
	if (light.type != LightType_Directional)
	{
		pointToLight = light.position - inGlobalPos;
	}
	else
	{
		pointToLight = -light.direction;
	}

	vec3 l = normalize(pointToLight);
	vec3 h = normalize(l + v);  // Half vector between both l and v

	// TODO: Changing the clamp value to 0 here results in very strange colour values. This is NOT RIGHT.
	float NdotL = clamp(dot(n, l), 0.001, 1.0);
	float NdotV = clamp(dot(n, v), 0.0, 1.0);
	float NdotH = clamp(dot(n, h), 0.0, 1.0);
	float LdotH = clamp(dot(l, h), 0.0, 1.0);
	float VdotH = clamp(dot(v, h), 0.0, 1.0);

	vec3 color;

	if (NdotL > 0. || NdotV > 0.) {
		vec3 intensity = getLightIntensity(light, l);

		// Obtain final intensity as reflectance (BRDF) scaled by the energy of the light (cosine law)
		vec3 diffuseContrib = intensity * NdotL * BRDF_lambertian(materialInfo.f0, f90, materialInfo.diffuseColor, VdotH);
		vec3 specContrib = intensity * NdotL * BRDF_specularGGX(materialInfo.f0, f90, materialInfo.alphaRoughness, VdotH, NdotL, NdotV, NdotH);

		// Finally, combind the diffuse and specular contributions
		color = diffuseContrib + specContrib;
	}

	return color;
}

vec3 getPBRMetallicRoughnessColor(Material material, vec4 baseColor) {

	// Metallic and Roughness material properties are packed together
	// In glTF, these factors can be specified by fixed scalar values
	// or from a metallic-roughness map
	float perceptualRoughness = material.roughnessFactor;
	float metalness = material.metallicFactor;

	if (material.physicalDescriptorTextureID == NO_TEXTURE) {
		perceptualRoughness = clamp(perceptualRoughness, 0., 1.0);
		metalness = clamp(metalness, 0., 1.0);
	} else {
		// Roughness is stored in the 'g' channel, metallic is stored in the 'a' channel, as per
		// https://github.com/ARM-software/astc-encoder/blob/main/Docs/Encoding.md#encoding-1-4-component-data
		vec4 mrSample = texture(textures[material.physicalDescriptorTextureID], inUV);

		// Roughness is authored as perceptual roughness; as is convention,
		// convert to material roughness by squaring the perceptual roughness [2].
		perceptualRoughness = mrSample.g * perceptualRoughness;
		metalness = mrSample.a * metalness;
	}

	// Get the diffuse colour
	vec3 f0 = mix(vec3(0.4), baseColor.rgb, metalness);
	vec3 diffuseColor = mix(baseColor.rgb, vec3(0.), metalness);

	float alphaRoughness = perceptualRoughness * perceptualRoughness;
	vec3 specularColor = mix(f0, baseColor.rgb, metalness);

	// Get the normal
	vec3 n = (material.normalTextureID == NO_TEXTURE) ? normalize(inNormal) : getNormal(material.normalTextureID);

	// Get the view vector - from surface point to camera
	vec3 v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGlobalPos);
	float NdotV = clamp(abs(dot(n, v)), 0.001, 1.0);

	// TODO: Is this correct?
	vec3 reflection = -normalize(reflect(v, n));
	reflection.y *= -1.;

	MaterialInfo materialInfo = MaterialInfo(
		f0,
		f90,
		diffuseColor,
		specularColor,
		alphaRoughness,
		perceptualRoughness,
		NdotV
	);

	// Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
	// vec3 color = getIBLContribution(materialInfo, n, reflection) * sceneData.params.x;

	vec3 color = vec3(0.);

	// Apply optional PBR terms for additional (optional) shading
	if (material.occlusionTextureID != NO_TEXTURE) {
		// Occlusion is stored in the 'g' channel as per:
		// https://github.com/ARM-software/astc-encoder/blob/main/Docs/Encoding.md#encoding-1-4-component-data
		float ao = texture(textures[material.occlusionTextureID], inUV).g;
		// color = color * ao;
	}

    // Walk through each light and add its color contribution.

	// Now add a spot light;
	Light pointLight = Light(
		vec3(0., 0., 0.),  // no direction
		3.0, // range

		vec3(1., 1., 1.), // color
		5.0, // intensity

		vec3(-2., 2., 0.), // position
		0.,

		0.,
		LightType_Point
	);

	// Now add a spot light;
	Light spotLight = Light(
		vec3(0., 0., -1.), // direction, transformed along negative Z
		10.0, // range

		vec3(1., 1., 1.), // color
		5.0, // intensity

		vec3(0., 2., 0.), // position
		cos(0.), // innerConeCos

		cos(0.7853982), // outerConeCos
		LightType_Spot
	);

	color += getLightContribution(materialInfo, n, v, pointLight);

	if (material.emissiveTextureID != NO_TEXTURE) {
		vec3 emissive = texture(textures[material.emissiveTextureID], inUV).rgb;
		color += emissive;
	}

	return color;
}

void main() {
	// Start by setting the output color to a familiar "error" magenta.
	outColor = ERROR_MAGENTA;

	// Retrieve the material from the buffer.
	Material material = materialBuffer.materials[inMaterialID];

	// Determine the base color
	vec4 baseColor;

	if (material.baseColorTextureID == NO_TEXTURE) {
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
		outColor.rgb = getPBRMetallicRoughnessColor(material, baseColor);
	} else if (material.workflow == PBR_WORKFLOW_UNLIT) {
		outColor = baseColor;
	}

	// Finally, tonemap the color.
	// outColor = tonemap(outColor);

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
				vec3 n = (material.normalTextureID == NO_TEXTURE) ? normalize(inNormal) : getNormal(material.normalTextureID);
				outColor.rgb = n * 0.5 + 0.5;
				break;
			case 3:
				outColor.rgb = (material.occlusionTextureID == NO_TEXTURE) ? vec3(0.0f) : texture(textures[material.occlusionTextureID], inUV).ggg;
				break;
			case 4:
				outColor.rgb = (material.emissiveTextureID == NO_TEXTURE) ?  vec3(0.0f) : texture(textures[material.emissiveTextureID], inUV).rgb;
				break;
			case 5:
				outColor.rgb = (material.physicalDescriptorTextureID == NO_TEXTURE) ? vec3(0.0) : texture(textures[material.physicalDescriptorTextureID], inUV).ggg;
				break;
			case 6:
				outColor.rgb = (material.physicalDescriptorTextureID == NO_TEXTURE) ? vec3(0.0) : texture(textures[material.physicalDescriptorTextureID], inUV).aaa;
				break;
		}
		outColor = outColor;
	}
}