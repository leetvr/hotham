// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460

#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_shader_explicit_arithmetic_types_int16 : require

#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

// Inputs
layout (location = 0) in vec3 inGosPos;
layout (location = 1) in vec2 inUV;
// layout (location = 2) in vec3 inNormal;

// Textures
layout (set = 0, binding = 3) uniform sampler2D textures[];
layout (set = 0, binding = 4) uniform samplerCube cubeTextures[];

#define DEFAULT_EXPOSURE 1.0
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS F16(10)
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA f16vec3(1, 0, 1)

#define TEXTURE_FLAG_HAS_PBR_TEXTURES 1
#define TEXTURE_FLAG_HAS_NORMAL_MAP 2
#define TEXTURE_FLAG_HAS_AO_TEXTURE 4
#define TEXTURE_FLAG_HAS_EMISSION_TEXTURE 8

layout( push_constant ) uniform constants
{
    uint textureFlags;
    uint baseTextureID;
} material;

// The default index of refraction of 1.5 yields a dielectric normal incidence reflectance (eg. f0) of 0.04
#define DEFAULT_F0 V16(0.04)

// Fast approximation of ACES tonemap
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
f16vec3 toneMapACES_Narkowicz(f16vec3 color) {
    const float16_t A = F16(2.51);
    const float16_t B = F16(0.03);
    const float16_t C = F16(2.43);
    const float16_t D = F16(0.59);
    const float16_t E = F16(0.14);
    return clamp((color * (A * color + B)) / (color * (C * color + D) + E), F16(0), F16(1));
}

f16vec3 tonemap(const f16vec3 color) {
    return toneMapACES_Narkowicz(color);
}

// Get normal, tangent and bitangent vectors.
f16vec3 getNormal() {
    vec3 N = normalize(vec3(1));

    f16vec3 textureNormal;
    textureNormal.xy = f16vec2(texture(textures[material.baseTextureID + 2], inUV).ga) * F16(2) - F16(1);
    textureNormal.z = sqrt(F16(1) - dot(textureNormal.xy, textureNormal.xy));

    // We compute the tangents on the fly because it is faster, presumably because it saves bandwidth.
    // See http://www.thetenthplanet.de/archives/1180 for an explanation of how this works
    // and a little bit about why it is better than using precomputed tangents.
    // Note however that we are using a slightly different formulation with coordinates in
    // globally oriented stage space instead of view space and we rely on the UV map not being too distorted.
    vec3 dGosPosDx = dFdx(inGosPos);
    vec3 dGosPosDy = dFdy(inGosPos);
    vec2 dUvDx = dFdx(inUV);
    vec2 dUvDy = dFdy(inUV);

    vec3 T = normalize(dGosPosDx * dUvDy.t - dGosPosDy * dUvDx.t);
    vec3 B = normalize(cross(N, T));
    mat3 TBN = mat3(T, B, N);

    return V16(normalize(TBN * textureNormal));
}

// Calculation of the lighting contribution from an optional Image Based Light source.
f16vec3 getIBLContribution(f16vec3 F0, float16_t perceptualRoughness, f16vec3 diffuseColor, f16vec3 reflection, float16_t NdotV) {
    float16_t lod = perceptualRoughness * DEFAULT_CUBE_MIPMAP_LEVELS - F16(1);

    f16vec2 brdfSamplePoint = clamp(f16vec2(NdotV, perceptualRoughness), f16vec2(0), f16vec2(1.0));
    f16vec2 f_ab = f16vec2(texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint)).rg;
    f16vec3 specularLight = V16(textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod).r);

    // see https://bruop.github.io/ibl/#single_scattering_results at Single Scattering Results
    // Roughness dependent fresnel, from Fdez-Aguera
    f16vec3 Fr = max(f16vec3(1.0 - perceptualRoughness), F0) - F0;
    f16vec3 k_S = F0 + Fr * pow(F16(1.0) - NdotV, F16(5.0));
    f16vec3 FssEss = k_S * f_ab.x + f_ab.y;

    f16vec3 specular = specularLight * FssEss;

    // Multiple scattering, from Fdez-Aguera
    f16vec3 diffuseLight = V16(textureLod(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], reflection, lod).rgb);

    f16vec3 diffuse = diffuseLight * diffuseColor * BRDF_LAMBERTIAN;

    return diffuse + specular;
}

f16vec3 getLightContribution(f16vec3 f0, float16_t alphaRoughness, f16vec3 diffuseColor, vec3 n, vec3 v, float16_t NdotV, Light light, float16_t ao) {
    // Get a vector between this point and the light.
    vec3 pointToLight;
    if (light.type != LightType_Directional) {
        pointToLight = light.position - inGosPos;
    } else {
        pointToLight = -light.direction;
    }

    vec3 l = normalize(pointToLight);
    vec3 h = normalize(l + v);  // Half vector between both l and v

    float16_t NdotL = F16(clamp(dot(n, l), 0, 1));
    float16_t NdotH = F16(clamp(dot(n, h), 0, 1));
    float16_t LdotH = F16(clamp(dot(l, h), 0, 1));

    f16vec3 color;

    if (NdotL > 0. || NdotV > 0.) {
        float16_t attenuation = getLightAttenuation(light, pointToLight, l);

        f16vec3 diffuseContrib = diffuseColor * BRDF_LAMBERTIAN;
        f16vec3 specContrib = BRDF_specular(f0, alphaRoughness, V16(h), V16(n), NdotV, NdotL, NdotH, LdotH);

        // Finally, combine the diffuse and specular contributions
        color = (diffuseContrib + specContrib) * (F16(light.intensity) * attenuation * NdotL * ao);
    }

    return color;
}

f16vec3 getPBRMetallicRoughnessColor() {
    f16vec3 baseColor;
    f16vec3 amrSample;
    f16vec3 normal;

    if ((material.textureFlags & TEXTURE_FLAG_HAS_PBR_TEXTURES) != 0) {
        baseColor = V16(texture(textures[material.baseTextureID], inUV).rgb);
        amrSample = V16(texture(textures[material.baseTextureID + 1], inUV).rgb);
    } else {
        baseColor = f16vec3(1);
        amrSample = f16vec3(1, 0.5, 0.5);
    }

    if ((material.textureFlags & TEXTURE_FLAG_HAS_NORMAL_MAP) != 0) {
        normal = getNormal();
    } else {
        // As per the glTF spec:
        // The textures for metalness and roughness properties are packed together in a single texture called metallicRoughnessTexture.
        // Its green channel contains roughness values and its blue channel contains metalness values.
        normal = normalize(V16(1));
    }

    float16_t perceptualRoughness = clamp(amrSample.g, MEDIUMP_FLT_MIN, F16(1.0));
    float16_t metalness = amrSample.b;

    // Get this material's f0
    f16vec3 f0 = mix(DEFAULT_F0, baseColor, metalness);

    // Get the diffuse color
    f16vec3 diffuseColor = baseColor * (F16(1.0) - metalness);

    // Roughness is authored as perceptual roughness; as is convention,
    // convert to material roughness by squaring the perceptual roughness
    float16_t alphaRoughness = perceptualRoughness * perceptualRoughness;

    // Get the view vector - from surface point to camera
    // IMPORTANT: Keep this as 32bit precision
    vec3 v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGosPos);

    // Get NdotV and reflection
    float16_t NdotV = saturate(F16(abs(dot(normal, V16(v)))));

    // Occlusion is stored in the 'r' channel as per the glTF spec
    float16_t ao = amrSample.r;

    // Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
    f16vec3 color;
    if (sceneData.params.x > 0.) {
        f16vec3 reflection = normalize(reflect(V16(-v), V16(normal)));
        color = getIBLContribution(f0, perceptualRoughness, diffuseColor, reflection, NdotV) * ao * F16(sceneData.params.x);
    }


    // Walk through each light and add its color contribution.
    // Qualcomm's documentation suggests that loops are undesirable, so we do branches instead.
    // Since these values are uniform, they shouldn't have too high of a penalty.
    if (sceneData.lights[0].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, normal, v, NdotV, sceneData.lights[0], ao);
    }
    if (sceneData.lights[1].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, normal, v, NdotV, sceneData.lights[1], ao);
    }
    if (sceneData.lights[2].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, normal, v, NdotV, sceneData.lights[2], ao);
    }
    // if (sceneData.lights[3].type != NOT_PRESENT) {
    //     color += getLightContribution(f0, alphaRoughness, diffuseColor, f16n, v, NdotV, sceneData.lights[3]);
    // }

    // Add emission, if present
    if ((material.textureFlags & TEXTURE_FLAG_HAS_EMISSION_TEXTURE) > 0) {
        color += V16(texture(textures[material.baseTextureID + 3], inUV)).rgb;
    }

    return color;
}



// Outputs
layout (location = 0) out vec4 outColor;

void main() {
    // f16vec3 color = getPBRMetallicRoughnessColor();

    // Finally, tonemap the color.
    // outColor.rgb = tonemap(color);
    // outColor.a = 1;
    outColor = vec4(1, 0, 0, 1);
}
