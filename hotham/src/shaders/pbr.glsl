#define DEFAULT_IBL_SCALE float16_t(0.4)
#define DEFAULT_CUBE_MIPMAP_LEVELS 10
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA f16vec4(1., 0., 1., 1.)

struct Material {
    vec4 baseColorFactor;
    uint workflow;
    uint baseColorTextureID;
    uint metallicRoughnessTextureID;
    uint normalTextureID;
    uint occlusionTextureID;
    uint emissiveTextureID;
    float metallicFactor;
    float roughnessFactor;
    float alphaMask;
    float alphaMaskCutoff;
};

const float PBR_WORKFLOW_METALLIC_ROUGHNESS = 0.0;
const float PBR_WORKFLOW_UNLIT = 1.0;

// The default index of refraction of 1.5 yields a dielectric normal incidence reflectance (eg. f0) of 0.04
const f16vec3 DEFAULT_F0 = f16vec3(0.04);

// Fast approximation of ACES tonemap
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
f16vec3 tonemap(f16vec3 color) {
    const float16_t A = float16_t(2.51);
    const float16_t B = float16_t(0.03);
    const float16_t C = float16_t(2.43);
    const float16_t D = float16_t(0.59);
    const float16_t E = float16_t(0.14);
    return clamp((color * (A * color + B)) / (color * (C * color + D) + E), float16_t(0.0), float16_t(1.0));
}

// Get normal, tangent and bitangent vectors.
f16vec3 getNormal(uint normalTextureID) {
    f16vec3 N = normalize(f16vec3(inNormal));
    if (normalTextureID == NOT_PRESENT) {
        return N;
    }

    f16vec3 textureNormal;
    textureNormal.xy = f16vec2(texture(textures[normalTextureID], inUV).ga) * float16_t(2.0) - float16_t(1.0);
    textureNormal.z = sqrt(float16_t(1) - dot(textureNormal.xy, textureNormal.xy));

    // We compute the tangents on the fly because it is faster, presumably because it saves bandwidth.
    // See http://www.thetenthplanet.de/archives/1180 for an explanation of how this works
    // and a little bit about why it is better than using precomputed tangents.
    // Note however that we are using a slightly different formulation with coordinates in
    // globally oriented stage space instead of view space and we rely on the UV map not being too distorted.
    f16vec3 dGosPosDx = f16vec3(dFdx(inGosPos));
    f16vec3 dGosPosDy = f16vec3(dFdy(inGosPos));
    // These give problems when trying to use float16_t.
    float dVDx = dFdx(inUV.t);
    float dVDy = dFdy(inUV.t);

    f16vec3 T = f16vec3(normalize(dGosPosDx * dVDy - dGosPosDy * dVDx));
    f16vec3 B = normalize(cross(N, T));
    f16mat3 TBN = f16mat3(T, B, N);

    return f16vec3(normalize(TBN * textureNormal));
}

// Calculation of the lighting contribution from an optional Image Based Light source.
f16vec3 getIBLContribution(f16vec3 F0, float16_t perceptualRoughness, f16vec3 diffuseColor, f16vec3 reflection, float16_t NdotV) {
    float16_t lod = perceptualRoughness * float16_t(DEFAULT_CUBE_MIPMAP_LEVELS - 1);

    f16vec2 brdfSamplePoint = clamp(f16vec2(NdotV, perceptualRoughness), f16vec2(0.0, 0.0), f16vec2(1.0, 1.0));
    f16vec2 f_ab = f16vec2(texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint).rg);

    f16vec3 specularLight = f16vec3(textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod).rgb);

    // see https://bruop.github.io/ibl/#single_scattering_results at Single Scattering Results
    // Roughness dependent fresnel, from Fdez-Aguera
    f16vec3 Fr = max(f16vec3(1.0 - perceptualRoughness), F0) - F0;
    f16vec3 k_S = F0 + Fr * pow(float16_t(1.0) - NdotV, float16_t(5.0));
    f16vec3 FssEss = k_S * f_ab.x + f_ab.y;

    f16vec3 specular = specularLight * FssEss;

    // Multiple scattering, from Fdez-Aguera
    f16vec3 diffuseLight = f16vec3(textureLod(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], reflection, lod).rgb);
    float16_t Ems = (float16_t(1.0) - (f_ab.x + f_ab.y));
    f16vec3 F_avg = F0 + (float16_t(1.0) - F0) / float16_t(21.0);
    f16vec3 FmsEms = Ems * FssEss * F_avg / (float16_t(1.0) - F_avg * Ems);
    f16vec3 k_D = diffuseColor * (float16_t(1.0) - FssEss + FmsEms); // we use +FmsEms as indicated by the formula in the blog post (might be a typo in the implementation)

    f16vec3 diffuse = (FmsEms + k_D) * diffuseLight;

    return diffuse + specular;
}

f16vec3 getLightContribution(f16vec3 F0, float16_t alphaRoughness, f16vec3 diffuseColor, f16vec3 n, f16vec3 v, float16_t NdotV, Light light) {
    // Get a vector between this point and the light.
    f16vec3 pointToLight;
    if (light.type != LightType_Directional) {
        pointToLight = f16vec3(light.position - inGosPos);
    } else {
        pointToLight = -f16vec3(light.direction);
    }

    f16vec3 l = normalize(pointToLight);
    f16vec3 h = normalize(l + v);  // Half vector between both l and v

    float16_t NdotL = clamp(dot(n, l), float16_t(0.0), float16_t(1.0));
    float16_t NdotH = clamp(dot(n, h), float16_t(0.0), float16_t(1.0));
    float16_t VdotH = clamp(dot(v, h), float16_t(0.0), float16_t(1.0));

    f16vec3 color = f16vec3(0);

    if (NdotL > 0. || NdotV > 0.) {
        f16vec3 intensity = getLightIntensity(light, pointToLight);

        // Obtain final intensity as reflectance (BRDF) scaled by the energy of the light (cosine law)
        f16vec3 diffuseContrib = intensity * NdotL * BRDF_lambertian(F0, diffuseColor, VdotH);
        f16vec3 specContrib = intensity * NdotL * BRDF_specularGGX(F0, alphaRoughness, VdotH, NdotL, NdotV, NdotH);

        // Finally, combine the diffuse and specular contributions
        color = diffuseContrib + specContrib;
    }

    return color;
}

f16vec3 getPBRMetallicRoughnessColor(Material material, f16vec4 baseColor) {
    // Metallic and Roughness material properties are packed together
    // In glTF, these factors can be specified by fixed scalar values
    // or from a metallic-roughness map
    float16_t perceptualRoughness = float16_t(material.roughnessFactor);
    float16_t metalness = float16_t(material.metallicFactor);

    if (material.metallicRoughnessTextureID == NOT_PRESENT) {
        perceptualRoughness = clamp(perceptualRoughness, float16_t(0.), float16_t(1.0));
        metalness = clamp(metalness, float16_t(0.), float16_t(1.0));
    } else {
        // As per the glTF spec:
        // The textures for metalness and roughness properties are packed together in a single texture called metallicRoughnessTexture.
        // Its green channel contains roughness values and its blue channel contains metalness values.
        // TODO: Use f16vec2 for this since only red and green is used.
        f16vec4 mrSample = f16vec4(texture(textures[material.metallicRoughnessTextureID], inUV));

        perceptualRoughness = clamp(mrSample.g * perceptualRoughness, float16_t(0.0), float16_t(1.0));
        metalness = clamp(mrSample.b * metalness, float16_t(0.0), float16_t(1.0));
    }

    // Get this material's f0
    f16vec3 f0 = mix(f16vec3(DEFAULT_F0), baseColor.rgb, metalness);

    // Get the diffuse color
    f16vec3 diffuseColor = mix(baseColor.rgb, f16vec3(0.), metalness);

    // Roughness is authored as perceptual roughness; as is convention,
    // convert to material roughness by squaring the perceptual roughness
    float16_t alphaRoughness = perceptualRoughness * perceptualRoughness;

    // Get the view vector - from surface point to camera
    f16vec3 v = normalize(f16vec3(sceneData.cameraPosition[gl_ViewIndex].xyz - inGosPos));

    // Get the normal
    f16vec3 n = getNormal(material.normalTextureID);

    // Get NdotV and reflection
    float16_t NdotV = clamp(abs(dot(n, v)), float16_t(0.), float16_t(1.0));
    f16vec3 reflection = normalize(reflect(-v, n));

    // Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
    f16vec3 color;
    if (sceneData.params.x > 0.) {
        color = getIBLContribution(f0, perceptualRoughness, diffuseColor, reflection, NdotV) * float16_t(sceneData.params.x);
    } else {
        color = f16vec3(0.);
    }

    // Apply ambient occlusion, if present.
    if (material.occlusionTextureID != NOT_PRESENT) {
        // Occlusion is stored in the 'r' channel as per the glTF spec
        float16_t ao = float16_t(texture(textures[material.occlusionTextureID], inUV).r);
        color = color * ao;
    }

    // Walk through each light and add its color contribution.
    // Qualcomm's documentation suggests that loops are undesirable, so we do branches instead.
    // Since these values are uniform, they shouldn't have too high of a penalty.
    if (sceneData.lights[0].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[0]);
    }
    if (sceneData.lights[1].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[1]);
    }
    if (sceneData.lights[2].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[2]);
    }
    if (sceneData.lights[3].type != NOT_PRESENT) {
        color += getLightContribution(f0, alphaRoughness, diffuseColor, n, v, NdotV, sceneData.lights[3]);
    }

    // Add emission, if present
    if (material.emissiveTextureID != NOT_PRESENT) {
        color += f16vec3(texture(textures[material.emissiveTextureID], inUV).rgb);
    }

    return color;
}
