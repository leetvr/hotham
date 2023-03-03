#define DEFAULT_EXPOSURE 1.0
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS F16(10)
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA vec4(1., 0., 1., 1.)

#define MATERIAL_FLAG_HAS_BASE_COLOR_TEXTURE 1
#define MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE 2
#define MATERIAL_FLAG_HAS_NORMAL_TEXTURE 4
#define MATERIAL_FLAG_HAS_AO_TEXTURE 8
#define MATERIAL_FLAG_HAS_EMISSION_TEXTURE 16
#define PBR_WORKFLOW_UNLIT 32

// The default index of refraction of 1.5 yields a dielectric normal incidence reflectance (eg. f0) of 0.04
#define DEFAULT_F0 V16(0.04)

// Textures
layout (set = 0, binding = 3) uniform sampler2D textures[10000];
layout (set = 0, binding = 4) uniform samplerCube cubeTextures[100];

// Material
layout( push_constant ) uniform constants
{
    uint flagsAndBaseTextureID;
    uint packedBaseColor;
    uint packedMetallicRoughnessFactor;
} material;

// Store the unpacked material in globals to avoid copying when calling functions.
uint materialFlags;
uint baseTextureID;

// Common variables used throughout lighting equations
vec3 pos;   // pos
vec3 n;     // normal
vec3 v;     // view vector
vec2 uv;    // inUV

// Calculation of the lighting contribution from an optional Image Based Light source.
f16vec3 getIBLContribution(f16vec3 F0, float16_t perceptualRoughness, f16vec3 diffuseColor, f16vec3 reflection, float16_t NdotV) {
    float16_t lod = perceptualRoughness * DEFAULT_CUBE_MIPMAP_LEVELS - F16(1);

    f16vec2 brdfSamplePoint = clamp(f16vec2(NdotV, perceptualRoughness), f16vec2(0), f16vec2(1.0));
    f16vec2 f_ab = f16vec2(texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint)).rg;
    f16vec3 specularLight = V16(textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod));

    // see https://bruop.github.io/ibl/#single_scattering_results at Single Scattering Results
    // Roughness dependent fresnel, from Fdez-Aguera
    f16vec3 Fr = max(f16vec3(1.0 - perceptualRoughness), F0) - F0;
    f16vec3 k_S = F0 + Fr * pow(F16(1.0) - NdotV, F16(5.0));
    f16vec3 FssEss = k_S * f_ab.x + f_ab.y;

    f16vec3 specular = specularLight * FssEss;

    // Multiple scattering, from Fdez-Aguera
    f16vec3 diffuseLight = V16(textureLod(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], reflection, lod));

    f16vec3 diffuse = diffuseLight * diffuseColor * BRDF_LAMBERTIAN;

    return diffuse + specular;
}

f16vec3 getLightContribution(f16vec3 f0, float16_t alphaRoughness, f16vec3 diffuseColor, float16_t NdotV, Light light) {
    // Get a vector between this point and the light.
    vec3 pointToLight;
    if (light.type != LightType_Directional) {
        pointToLight = light.position - pos;
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
        color = (diffuseContrib + specContrib) * (F16(light.intensity) * attenuation * NdotL);
    }

    return color;
}

f16vec3 getPBRMetallicRoughnessColor(f16vec3 baseColor) {
    f16vec3 amrSample;

    if ((materialFlags & MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) {
        amrSample = V16(texture(textures[baseTextureID + 1], uv).rgb);
    } else {
        // If we don't have a metallic roughness texture, unpack the factors from the material.
        // Note the awkward swizzle: the variable name is "metallicRoughness", indicating that the
        // vector (x, y) is (metallic, roughness). However, we need to be consistent with the
        // channel order of the metallicRoughness texture, which is (g, b) - (roughness, metallic).
        amrSample.gb = f16vec2(unpackUnorm4x8(
            material.packedMetallicRoughnessFactor).yx);
    }

    // As per the glTF spec:
    // The textures for metalness and roughness properties are packed together in a single texture called metallicRoughnessTexture.
    // Its green channel contains roughness values and its blue channel contains metalness values.
    float16_t perceptualRoughness = clamp(amrSample.g, MEDIUMP_FLT_MIN, F16(1.0));
    float16_t metalness = amrSample.b;

    // Get this material's f0
    f16vec3 f0 = mix(DEFAULT_F0, baseColor, metalness);

    // Get the diffuse color
    f16vec3 diffuseColor = baseColor * (F16(1.0) - metalness);

    // Roughness is authored as perceptual roughness; as is convention,
    // convert to material roughness by squaring the perceptual roughness
    float16_t alphaRoughness = perceptualRoughness * perceptualRoughness;

    // Get NdotV and reflection
    float16_t NdotV = saturate(F16(abs(dot(n, v))));

    // Ambient Occlusion is stored in the 'r' channel as per the glTF spec
    float16_t ao;
    if ((materialFlags & MATERIAL_FLAG_HAS_AO_TEXTURE) != 0) {
        ao  = amrSample.r;
    } else {
        ao = F16(1);
    }

    // Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
    f16vec3 color;
    if (sceneData.params.x > 0.) {
        f16vec3 reflection = normalize(reflect(V16(-v), V16(n)));
        color = getIBLContribution(f0, perceptualRoughness, diffuseColor, reflection, NdotV) * ao * F16(sceneData.params.x);
    } else {
        // If there is no IBL, set color to 0 to handle the edge-case of having no lights at all.
        color = V16(0.0);
    }

    // Walk through each light and add its color contribution.
    // Qualcomm's documentation suggests that loops are undesirable, so we do branches instead.
    // Since these values are uniform, they shouldn't have too high of a penalty.
    if (sceneData.lights[0].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, NdotV, sceneData.lights[0]);
    }
    if (sceneData.lights[1].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, NdotV, sceneData.lights[1]);
    }
    if (sceneData.lights[2].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, NdotV, sceneData.lights[2]);
    }
    if (sceneData.lights[3].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, NdotV, sceneData.lights[3]);
    }

    // Add emission, if present
    if ((materialFlags & MATERIAL_FLAG_HAS_EMISSION_TEXTURE) > 0) {
        color += V16(texture(textures[baseTextureID + 3], uv)).rgb;
    }

    return color;
}

// Fast approximation of ACES tonemap
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
f16vec3 toneMapACES_Narkowicz(const f16vec3 color) {
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
