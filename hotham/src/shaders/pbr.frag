// PBR shader based on Sasche Williems' implementation:
// https://github.com/SaschaWillems/Vulkan-glTF-PBR
// Which in turn was based on https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460
#include "common.glsl"


// Inputs
layout (location = 0) in vec4 inWorldPos;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) flat in uint inMaterialID;

// Outputs
layout(location = 0) out vec4 oColor;

const float PI = 3.14159265359;

vec3 F0 = vec3(0.04);

// [0] Frensel Schlick
vec3 F_Schlick(vec3 f0, float f90, float u)
{
	return f0 + (f90 - f0) * pow(1.0 - u, 5.0);
}

// [1] IBL Defuse Irradiance
vec3 F_Schlick_Roughness(vec3 F0, float cos_theta, float roughness)
{
	return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(1.0 - cos_theta, 5.0);
}

// [0] Diffuse Term
float Fr_DisneyDiffuse(float NdotV, float NdotL, float LdotH, float roughness)
{
	float E_bias        = 0.0 * (1.0 - roughness) + 0.5 * roughness;
	float E_factor      = 1.0 * (1.0 - roughness) + (1.0 / 1.51) * roughness;
	float fd90          = E_bias + 2.0 * LdotH * LdotH * roughness;
	vec3  f0            = vec3(1.0);
	float light_scatter = F_Schlick(f0, fd90, NdotL).r;
	float view_scatter  = F_Schlick(f0, fd90, NdotV).r;
	return light_scatter * view_scatter * E_factor;
}

// [0] Specular Microfacet Model
float V_SmithGGXCorrelated(float NdotV, float NdotL, float roughness)
{
	float alphaRoughnessSq = roughness * roughness;

	float GGXV = NdotL * sqrt(NdotV * NdotV * (1.0 - alphaRoughnessSq) + alphaRoughnessSq);
	float GGXL = NdotV * sqrt(NdotL * NdotL * (1.0 - alphaRoughnessSq) + alphaRoughnessSq);

	float GGX = GGXV + GGXL;
	if (GGX > 0.0)
	{
		return 0.5 / GGX;
	}
	return 0.0;
}

// [0] GGX Normal Distribution Function
float D_GGX(float NdotH, float roughness)
{
	float alphaRoughnessSq = roughness * roughness;
	float f                = (NdotH * alphaRoughnessSq - NdotH) * NdotH + 1.0;
	return alphaRoughnessSq / (PI * f * f);
}

vec3 normal()
{
	vec3 pos_dx = dFdx(inWorldPos.xyz);
	vec3 pos_dy = dFdy(inWorldPos.xyz);
	vec3 st1    = dFdx(vec3(inUV, 0.0));
	vec3 st2    = dFdy(vec3(inUV, 0.0));
	vec3 T      = (st2.t * pos_dx - st1.t * pos_dy) / (st1.s * st2.t - st2.s * st1.t);
	vec3 N      = normalize(inNormal);
	T           = normalize(T - N * dot(N, T));
	vec3 B      = normalize(cross(N, T));
	mat3 TBN    = mat3(T, B, N);

#ifdef HAS_NORMAL_TEXTURE
	vec3 n = texture(normal_texture, inUV).rgb;
	return normalize(TBN * (2.0 * n - 1.0));
#else
	return normalize(TBN[2].xyz);
#endif
}

vec3 diffuse(vec3 albedo, float metallic)
{
	return albedo * (1.0 - metallic) + ((1.0 - metallic) * albedo) * metallic;
}

float saturate(float t)
{
	return clamp(t, 0.0, 1.0);
}

vec3 saturate(vec3 t)
{
	return clamp(t, 0.0, 1.0);
}

vec3 apply_directional_light(vec3 normal)
{
	vec3 world_to_light = -sceneData.lightDirection.xyz;

	world_to_light = normalize(world_to_light);

	float ndotl = clamp(dot(normal, world_to_light), 0.0, 1.0);

	return vec3(ndotl);
}

vec3 get_light_direction()
{
	return -sceneData.lightDirection.xyz;
}

void main(void)
{
	// vec3 position = vec3(0, 0, 0);

	float F90        = saturate(50.0 * F0.r);
	vec4  baseColor;
	Material material = materialBuffer.materials[inMaterialID];
	float roughness;
	float metallic;

	if (material.baseColorTextureID == NO_TEXTURE) {
		baseColor = material.baseColorFactor;
	} else {
		baseColor = texture(textures[material.baseColorTextureID], inUV);
	}

	if (material.metallicRoughnessTextureID == NO_TEXTURE) {
		roughness = material.roughnessFactor;
		metallic  = material.metallicFactor;
	} else {
		float roughness = saturate(texture(textures[material.metallicRoughnessTextureID], inUV).g);
		float metallic  = saturate(texture(textures[material.metallicRoughnessTextureID], inUV).b);
	}

	vec3  N     = normal();
	vec3  V     = normalize(sceneData.cameraPosition[gl_ViewIndex] - inWorldPos).xyz;
	float NdotV = saturate(dot(N, V));

	vec3 LightContribution = vec3(0.0);
	vec3 diffuse_color     = baseColor.rgb * (1.0 - metallic);

	vec3 L = get_light_direction();
	vec3 H = normalize(V + L);

	float LdotH = saturate(dot(L, H));
	float NdotH = saturate(dot(N, H));
	float NdotL = saturate(dot(N, L));

	vec3  F   = F_Schlick(F0, F90, LdotH);
	float Vis = V_SmithGGXCorrelated(NdotV, NdotL, roughness);
	float D   = D_GGX(NdotH, roughness);
	vec3  Fr  = F * D * Vis;

	float Fd = Fr_DisneyDiffuse(NdotV, NdotL, LdotH, roughness);

	LightContribution += apply_directional_light(N) * (diffuse_color * (vec3(1.0) - F) * Fd + Fr);

	// [1] Tempory irradiance to fix dark metals
	// TODO: add specular irradiance for realistic metals
	vec3 irradiance  = vec3(0.5);
	vec3 ibl_diffuse = irradiance * baseColor.rgb;

	vec3 ambient_color = ibl_diffuse;

	oColor = vec4(0.3 * ambient_color + LightContribution, baseColor.a);
}