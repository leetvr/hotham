// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

// Inputs
layout (location = 0) in vec3 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) flat in uint inMaterialID;
layout (location = 3) in vec3 inNormal;

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

#define DEFAULT_EXPOSURE 1.0
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS 10
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA vec4(1., 0., 1., 1.)

layout( push_constant ) uniform constants
{
    uint baseTextureID;
} pc;

// The default index of refraction of 1.5 yields a dielectric normal incidence reflectance (eg. f0) of 0.04
const vec3 DEFAULT_F0 = vec3(0.04);

// Fast approximation of ACES tonemap
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
vec3 toneMapACES_Narkowicz(vec3 color) {
    const float A = 2.51;
    const float B = 0.03;
    const float C = 2.43;
    const float D = 0.59;
    const float E = 0.14;
    return clamp((color * (A * color + B)) / (color * (C * color + D) + E), 0.0, 1.0);
}

vec3 tonemap(vec3 color) {
    color *= DEFAULT_EXPOSURE;
    color = toneMapACES_Narkowicz(color.rgb);
    return color;
}

// Get normal, tangent and bitangent vectors.
vec3 getNormal() {
    vec3 N = normalize(inNormal);

    vec3 textureNormal;
    textureNormal.xy = texture(textures[pc.baseTextureID + 2], inUV).ga * 2.0 - 1.0;
    textureNormal.z = sqrt(1 - dot(textureNormal.xy, textureNormal.xy));

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

    return normalize(TBN * textureNormal);
}

// Calculation of the lighting contribution from an optional Image Based Light source.
vec3 getIBLContribution(vec3 F0, float perceptualRoughness, vec3 diffuseColor, vec3 reflection, float NdotV) {
    float lod = perceptualRoughness * float(DEFAULT_CUBE_MIPMAP_LEVELS - 1);

    vec2 brdfSamplePoint = clamp(vec2(NdotV, perceptualRoughness), vec2(0.0, 0.0), vec2(1.0, 1.0));
    vec2 f_ab = texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint).rg;
    vec3 specularLight = textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod).rgb;

    // see https://bruop.github.io/ibl/#single_scattering_results at Single Scattering Results
    // Roughness dependent fresnel, from Fdez-Aguera
    vec3 Fr = max(vec3(1.0 - perceptualRoughness), F0) - F0;
    vec3 k_S = F0 + Fr * pow(1.0 - NdotV, 5.0);
    vec3 FssEss = k_S * f_ab.x + f_ab.y;

    vec3 specular = specularLight * FssEss;

    // Multiple scattering, from Fdez-Aguera
    vec3 diffuseLight = textureLod(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], reflection, lod).rgb;
    float Ems = (1.0 - (f_ab.x + f_ab.y));
    vec3 F_avg = F0 + (1.0 - F0) / 21.0;
    vec3 FmsEms = Ems * FssEss * F_avg / (1.0 - F_avg * Ems);
    vec3 k_D = diffuseColor * (1.0 - FssEss + FmsEms); // we use +FmsEms as indicated by the formula in the blog post (might be a typo in the implementation)

    vec3 diffuse = (FmsEms + k_D) * diffuseLight;

    return diffuse + specular;
}

vec3 getLightContribution(vec3 F0, float alphaRoughness, vec3 diffuseColor, vec3 n, vec3 v, float NdotV, Light light) {
    // Get a vector between this point and the light.
    vec3 pointToLight;
    if (light.type != LightType_Directional) {
        pointToLight = light.position - inGosPos;
    } else {
        pointToLight = -light.direction;
    }

    vec3 l = normalize(pointToLight);
    vec3 h = normalize(l + v);  // Half vector between both l and v

    float NdotL = clamp(dot(n, l), 0.0, 1.0);
    float NdotH = clamp(dot(n, h), 0.0, 1.0);
    float VdotH = clamp(dot(v, h), 0.0, 1.0);

    vec3 color;

    if (NdotL > 0. || NdotV > 0.) {
        vec3 intensity = getLightIntensity(light, pointToLight);

        // Obtain final intensity as reflectance (BRDF) scaled by the energy of the light (cosine law)
        vec3 diffuseContrib = intensity * NdotL * BRDF_lambertian(F0, diffuseColor, VdotH);
        vec3 specContrib = intensity * NdotL * BRDF_specularGGX(F0, alphaRoughness, VdotH, NdotL, NdotV, NdotH);

        // Finally, combine the diffuse and specular contributions
        color = diffuseContrib + specContrib;
    }

    return color;
}

vec3 getPBRMetallicRoughnessColor() {
    vec3 baseColor = texture(textures[pc.baseTextureID], inUV).rgb;
    // As per the glTF spec:
    // The textures for metalness and roughness properties are packed together in a single texture called metallicRoughnessTexture.
    // Its green channel contains roughness values and its blue channel contains metalness values.
    vec4 amrSample = texture(textures[pc.baseTextureID + 1], inUV);

    float perceptualRoughness = clamp(amrSample.g, 0.0, 1.0);
    float metalness = clamp(amrSample.b, 0.0, 1.0);

    // Get this material's f0
    vec3 f0 = mix(DEFAULT_F0, baseColor.rgb, metalness);

    // Get the diffuse color
    vec3 diffuseColor = mix(baseColor.rgb, vec3(0.), metalness);

    // Roughness is authored as perceptual roughness; as is convention,
    // convert to material roughness by squaring the perceptual roughness
    float alphaRoughness = perceptualRoughness * perceptualRoughness;

    // Get the view vector - from surface point to camera
    // vec3 v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGosPos);
    vec3 v = normalize(vec3(0, 0.5, 0) - inGosPos);

    // Get the normal
    vec3 n = getNormal();

    // Get NdotV and reflection
    float NdotV = clamp(abs(dot(n, v)), 0., 1.0);
    vec3 reflection = normalize(reflect(-v, n));

    // Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
    vec3 color;
    // if (sceneData.params.x > 0.) {
        color = getIBLContribution(f0, perceptualRoughness, diffuseColor, reflection, NdotV);
    // } else {
    //     color = vec3(0.);
    // }

    // Occlusion is stored in the 'r' channel as per the glTF spec
    float ao = amrSample.r;
    color = color * ao;

    // Walk through each light and add its color contribution.
    // Qualcomm's documentation suggests that loops are undesirable, so we do branches instead.
    // Since these values are uniform, they shouldn't have too high of a penalty.
    Light light = Light(vec3(0.1612209, -0.7077924, 0.68777746), 0, vec3(1.), 10., vec3(0.), 0, 0, 0);
    color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, light);
    color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, light);
    color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, light);
    color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, light);
    // if (sceneData.lights[0].type != NOT_PRESENT) {
    //     color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[0]);
    // }
    // if (sceneData.lights[1].type != NOT_PRESENT) {
    //     color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[1]);
    // }
    // if (sceneData.lights[2].type != NOT_PRESENT) {
    //     color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[2]);
    // }
    // if (sceneData.lights[3].type != NOT_PRESENT) {
    //     color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[3]);
    // }

    // Add emission, if present
    vec3 emissive = texture(textures[pc.baseTextureID + 3], inUV).rgb;
    color += emissive;

    return color;
}



// Outputs
layout (location = 0) out vec4 outColor;

void main() {
    // Start by setting the output color to a familiar "error" magenta.
    outColor = ERROR_MAGENTA;

    outColor.rgb = getPBRMetallicRoughnessColor();

    // Finally, tonemap the color.
    outColor.rgb = tonemap(outColor.rgb);
}
