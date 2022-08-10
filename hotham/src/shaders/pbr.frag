// PBR shader based on Sascha Willems' implementation:
// https://github.com/SaschaWillems/Vulkan-glTF-PBR
// Which in turn was based on https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460
#include "common.glsl"

const float epsilon = 1e-6;
#define DEFAULT_EXPOSURE 4.5
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS 10
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1

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
struct PBRInfo
{
	float NdotL;                  // cos angle between normal and light direction
	float NdotV;                  // cos angle between normal and view direction
	float NdotH;                  // cos angle between normal and half vector
	float LdotH;                  // cos angle between light direction and half vector
	float VdotH;                  // cos angle between view direction and half vector
	float perceptualRoughness;    // roughness value, as authored by the model creator (input to shader)
	float metalness;              // metallic value at the surface
	vec3 reflectance0;            // full reflectance color (normal incidence angle)
	vec3 reflectance90;           // reflectance color at grazing angle
	float alphaRoughness;         // roughness mapped to a more linear change in the roughness (proposed by [2])
	vec3 diffuseColor;            // color contribution from diffuse lighting
	vec3 specularColor;           // color contribution from specular lighting
};

const float M_PI = 3.141592653589793;
const float c_MinRoughness = 0.04;

const float PBR_WORKFLOW_METALLIC_ROUGHNESS = 0.0;
const float PBR_WORKFLOW_SPECULAR_GLOSSINESS = 1.0f;
const float PBR_WORKFLOW_UNLIT = 2.0f;

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
vec3 getIBLContribution(PBRInfo pbrInputs, vec3 n, vec3 reflection)
{
	// Flip the y axis of the reflection vector; otherwise our cubemap will be upside down.
	// TODO: Fix this by pre-flipping the cubemaps.
	reflection.y *= -1.;
	float lod = pbrInputs.perceptualRoughness * float(DEFAULT_CUBE_MIPMAP_LEVELS -1);

	// retrieve a scale and bias to F0. See [1], Figure 3
	vec2 brdfSamplePoint = clamp(vec2(pbrInputs.NdotV, 1.0 - pbrInputs.perceptualRoughness), vec2(0.0, 0.0), vec2(1.0, 1.0));
	vec3 brdf = texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint).rgb;

	// Get diffuse and specular light values
	vec3 diffuseLight = texture(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], n).rgb;
	vec3 specularLight = textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod).rgb;

	// Multiply them by their respective inputs to get their final colors.
	vec3 diffuse = diffuseLight * pbrInputs.diffuseColor;
	vec3 specular = specularLight * (pbrInputs.specularColor * brdf.x + brdf.y);

	return diffuse + specular;
}

// Basic Lambertian diffuse
// Implementation from Lambert's Photometria https://archive.org/details/lambertsphotome00lambgoog
// See also [1], Equation 1
vec3 diffuse(PBRInfo pbrInputs)
{
	return pbrInputs.diffuseColor / M_PI;
}

// The following equation models the Fresnel reflectance term of the spec equation (aka F())
// Implementation of fresnel from [4], Equation 15
vec3 specularReflection(PBRInfo pbrInputs)
{
	return pbrInputs.reflectance0 + (pbrInputs.reflectance90 - pbrInputs.reflectance0) * pow(clamp(1.0 - pbrInputs.VdotH, 0.0, 1.0), 5.0);
}

// This calculates the specular geometric attenuation (aka G()),
// where rougher material will reflect less light back to the viewer.
// This implementation is based on [1] Equation 4, and we adopt their modifications to
// alphaRoughness as input as originally proposed in [2].
float geometricOcclusion(PBRInfo pbrInputs)
{
	float NdotL = pbrInputs.NdotL;
	float NdotV = pbrInputs.NdotV;
	float r = pbrInputs.alphaRoughness;

	float attenuationL = 2.0 * NdotL / (NdotL + sqrt(r * r + (1.0 - r * r) * (NdotL * NdotL)));
	float attenuationV = 2.0 * NdotV / (NdotV + sqrt(r * r + (1.0 - r * r) * (NdotV * NdotV)));
	return attenuationL * attenuationV;
}

// The following equation(s) model the distribution of microfacet normals across the area being drawn (aka D())
// Implementation from "Average Irregularity Representation of a Roughened Surface for Ray Reflection" by T. S. Trowbridge, and K. P. Reitz
// Follows the distribution function recommended in the SIGGRAPH 2013 course notes from EPIC Games [1], Equation 3.
float microfacetDistribution(PBRInfo pbrInputs)
{
	float roughnessSq = pbrInputs.alphaRoughness * pbrInputs.alphaRoughness;
	float f = (pbrInputs.NdotH * roughnessSq - pbrInputs.NdotH) * pbrInputs.NdotH + 1.0;
	return roughnessSq / (M_PI * f * f);
}

// Gets metallic factor from specular glossiness workflow inputs
float convertMetallic(vec3 diffuse, vec3 specular, float maxSpecular) {
	float perceivedDiffuse = sqrt(0.299 * diffuse.r * diffuse.r + 0.587 * diffuse.g * diffuse.g + 0.114 * diffuse.b * diffuse.b);
	float perceivedSpecular = sqrt(0.299 * specular.r * specular.r + 0.587 * specular.g * specular.g + 0.114 * specular.b * specular.b);
	if (perceivedSpecular < c_MinRoughness) {
		return 0.0;
	}
	float a = c_MinRoughness;
	float b = perceivedDiffuse * (1.0 - maxSpecular) / (1.0 - c_MinRoughness) + perceivedSpecular - 2.0 * c_MinRoughness;
	float c = c_MinRoughness - perceivedSpecular;
	float D = max(b * b - 4.0 * a * c, 0.0);
	return clamp((-b + sqrt(D)) / (2.0 * a), 0.0, 1.0);
}

void main()
{
	float perceptualRoughness;
	float metallic;
	vec3 diffuseColor;
	vec4 baseColor;

	vec3 f0 = vec3(0.04);
	Material material = materialBuffer.materials[inMaterialID];

	if (material.baseColorTextureID == NO_TEXTURE) {
		baseColor = material.baseColorFactor;
	} else {
		baseColor = texture(textures[material.baseColorTextureID], inUV) * material.baseColorFactor;
	}

	if (material.alphaMask == 1.0f) {
		if (baseColor.a < material.alphaMaskCutoff) {
			// TODO: Apparently Adreno GPUs don't like discarding.
			discard;
		}
	}

	if (material.workflow == PBR_WORKFLOW_METALLIC_ROUGHNESS) {
		// Metallic and Roughness material properties are packed together
		// In glTF, these factors can be specified by fixed scalar values
		// or from a metallic-roughness map
		perceptualRoughness = material.roughnessFactor;
		metallic = material.metallicFactor;
		if (material.physicalDescriptorTextureID == NO_TEXTURE) {
			perceptualRoughness = clamp(perceptualRoughness, c_MinRoughness, 1.0);
			metallic = clamp(metallic, 0.0, 1.0);
		} else {
			// Roughness is stored in the 'g' channel, metallic is stored in the 'a' channel, as per
			// https://github.com/ARM-software/astc-encoder/blob/main/Docs/Encoding.md#encoding-1-4-component-data
			vec4 mrSample = texture(textures[material.physicalDescriptorTextureID], inUV);

			// Roughness is authored as perceptual roughness; as is convention,
			// convert to material roughness by squaring the perceptual roughness [2].
			perceptualRoughness = mrSample.g * perceptualRoughness;
			metallic = mrSample.a * metallic;
		}
	}

	diffuseColor = baseColor.rgb * (vec3(1.0) - f0);
	diffuseColor *= 1.0 - metallic;

	float alphaRoughness = perceptualRoughness * perceptualRoughness;

	vec3 specularColor = mix(f0, baseColor.rgb, metallic);

	// Compute reflectance.
	float reflectance = max(max(specularColor.r, specularColor.g), specularColor.b);

	// For typical incident reflectance range (between 4% to 100%) set the grazing reflectance to 100% for typical fresnel effect.
	// For very low reflectance range on highly diffuse objects (below 4%), incrementally reduce grazing reflectance to 0%.
	float reflectance90 = clamp(reflectance * 25.0, 0.0, 1.0);
	vec3 specularEnvironmentR0 = specularColor.rgb;
	vec3 specularEnvironmentR90 = vec3(1.0, 1.0, 1.0) * reflectance90;

	vec3 n = (material.normalTextureID == NO_TEXTURE) ? normalize(inNormal) : getNormal(material.normalTextureID);
	vec3 v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGlobalPos);    // Vector from surface point to camera
	vec3 l = normalize(sceneData.lightDirection.xyz);     // Vector from surface point to light
	vec3 h = normalize(l+v);                        // Half vector between both l and v
	vec3 reflection = -normalize(reflect(v, n));

	// TODO: Is this correct?
	reflection.y *= -1.;

	float NdotL = clamp(dot(n, l), 0.001, 1.0);
	float NdotV = clamp(abs(dot(n, v)), 0.001, 1.0);
	float NdotH = clamp(dot(n, h), 0.0, 1.0);
	float LdotH = clamp(dot(l, h), 0.0, 1.0);
	float VdotH = clamp(dot(v, h), 0.0, 1.0);

	PBRInfo pbrInputs = PBRInfo(
		NdotL,
		NdotV,
		NdotH,
		LdotH,
		VdotH,
		perceptualRoughness,
		metallic,
		specularEnvironmentR0,
		specularEnvironmentR90,
		alphaRoughness,
		diffuseColor,
		specularColor
	);

	// Calculate the shading terms for the microfacet specular shading model
	vec3 F = specularReflection(pbrInputs);
	float G = geometricOcclusion(pbrInputs);
	float D = microfacetDistribution(pbrInputs);

	// Calculation of analytical lighting contribution
	vec3 diffuseContrib = (1.0 - F) * diffuse(pbrInputs);
	vec3 specContrib = F * G * D / (4.0 * NdotL * NdotV);

	// Obtain final intensity as reflectance (BRDF) scaled by the energy of the light (cosine law)
	vec3 color = NdotL * (diffuseContrib + specContrib);

	// Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
	color += getIBLContribution(pbrInputs, n, reflection) * sceneData.params.x;

	// Apply optional PBR terms for additional (optional) shading
	if (material.occlusionTextureID != NO_TEXTURE) {
		// Occlusion is stored in the 'g' channel as per:
		// https://github.com/ARM-software/astc-encoder/blob/main/Docs/Encoding.md#encoding-1-4-component-data
		float ao = texture(textures[material.occlusionTextureID], inUV).g;
		color = color * ao;
	}

	if (material.emissiveTextureID != NO_TEXTURE) {
		vec3 emissive = texture(textures[material.emissiveTextureID], inUV).rgb;
		color += emissive;
	}

	outColor = tonemap(vec4(color, 1.0));

	if (material.workflow == PBR_WORKFLOW_UNLIT) {
		outColor = baseColor;
		outColor.a = 1;
	}

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

	// "none", "Diff (l,n)", "F (l,h)", "G (l,v,h)", "D (h)", "Specular"
	if (sceneData.params.w > 0.0) {
		int index = int(sceneData.params.w);
		switch (index) {
			case 1:
				outColor.rgb = diffuseContrib;
				break;
			case 2:
				outColor.rgb = F;
				break;
			case 3:
				outColor.rgb = vec3(G);
				break;
			case 4:
				outColor.rgb = vec3(D);
				break;
			case 5:
				outColor.rgb = specContrib;
				break;
		}
	}
}
