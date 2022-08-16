const float epsilon = 1e-6;
#define DEFAULT_EXPOSURE 1.0
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS 10
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA vec4(1., 0., 1., 1.)

struct Material {
    vec4 baseColorFactor;
    vec4 emissiveFactor;
    uint workflow;
    uint baseColorTextureID;
    uint physicalDescriptorTextureID;
    uint normalTextureID;
    uint occlusionTextureID;
    uint emissiveTextureID;
    float metallicFactor;
    float roughnessFactor;
    float alphaMask;
    float alphaMaskCutoff;
};

// Encapsulate BRDF information about this material
struct MaterialInfo {
    vec3 f0;                      // full reflectance color (normal incidence angle)
    vec3 diffuseColor;            // color contribution from diffuse lighting
    vec3 specularColor;           // color contribution from specular lighting
    float alphaRoughness;         // roughness mapped to a more linear change in the roughness (proposed by [2])
    float perceptualRoughness;    // roughness value, as authored by the model creator (input to shader)
};

const float PBR_WORKFLOW_METALLIC_ROUGHNESS = 0.0;
const float PBR_WORKFLOW_UNLIT = 1.0;

// Anything less than 2% is physically impossible and is instead considered to be shadowing. Compare to "Real-Time-Rendering" 4th editon on page 325.
const vec3 f90 = vec3(1.0);

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
vec3 getNormal(uint normalTextureID) {
    vec3 n, t, b, ng;

    // Trivial TBN computation, present as vertex attribute.
    // Normalize eigenvectors as matrix is linearly interpolated.
    t = normalize(inTBN[0]);
    b = normalize(inTBN[1]);
    ng = normalize(inTBN[2]);

    if (normalTextureID != NOT_PRESENT) {
        vec3 ntex;
        ntex.xy = texture(textures[normalTextureID], inUV).ga * 2.0 - 1.0;
        ntex.z = sqrt(1 - dot(ntex.xy, ntex.xy));
        return normalize(mat3(t, b, ng) * ntex);
    } else {
        return ng;
    }
}

// Calculation of the lighting contribution from an optional Image Based Light source.
vec3 getIBLContribution(MaterialInfo materialInfo, vec3 n, vec3 reflection, float NdotV) {
    vec3 F0 = materialInfo.f0;
    float lod = materialInfo.perceptualRoughness * float(DEFAULT_CUBE_MIPMAP_LEVELS - 1);

    vec2 brdfSamplePoint = clamp(vec2(NdotV, materialInfo.perceptualRoughness), vec2(0.0, 0.0), vec2(1.0, 1.0));
    vec2 f_ab = texture(textures[BRDF_LUT_TEXTURE_ID], brdfSamplePoint).rg;

    vec3 specularLight = textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], reflection, lod).rgb;
    vec3 diffuseLight = textureLod(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], reflection, lod).rgb;

    // see https://bruop.github.io/ibl/#single_scattering_results at Single Scattering Results
    // Roughness dependent fresnel, from Fdez-Aguera
    vec3 Fr = max(vec3(1.0 - materialInfo.perceptualRoughness), F0) - F0;
    vec3 k_S = F0 + Fr * pow(1.0 - NdotV, 5.0);
    vec3 FssEss = k_S * f_ab.x + f_ab.y;

    vec3 specular = specularLight * FssEss;

    // Multiple scattering, from Fdez-Aguera
    float Ems = (1.0 - (f_ab.x + f_ab.y));
    vec3 F_avg = F0 + (1.0 - F0) / 21.0;
    vec3 FmsEms = Ems * FssEss * F_avg / (1.0 - F_avg * Ems);
    vec3 k_D = materialInfo.diffuseColor * (1.0 - FssEss + FmsEms); // we use +FmsEms as indicated by the formula in the blog post (might be a typo in the implementation)

    vec3 diffuse = (FmsEms + k_D) * diffuseLight;

    return diffuse + specular;
}

vec3 getLightContribution(MaterialInfo materialInfo, vec3 n, vec3 v, float NdotV, Light light) {
    // Get a vector between this point and the light.
    vec3 pointToLight;
    if (light.type != LightType_Directional) {
        pointToLight = light.position - inGlobalPos;
    } else {
        pointToLight = -light.direction;
    }

    vec3 l = normalize(pointToLight);
    vec3 h = normalize(l + v);  // Half vector between both l and v

    float NdotL = clamp(dot(n, l), 0.0, 1.0);
    float NdotH = clamp(dot(n, h), 0.0, 1.0);
    float LdotH = clamp(dot(l, h), 0.0, 1.0);
    float VdotH = clamp(dot(v, h), 0.0, 1.0);

    vec3 color;

    if (NdotL > 0. || NdotV > 0.) {
        vec3 intensity = getLightIntensity(light, pointToLight);

        // Obtain final intensity as reflectance (BRDF) scaled by the energy of the light (cosine law)
        vec3 diffuseContrib = intensity * NdotL * BRDF_lambertian(materialInfo.f0, f90, materialInfo.diffuseColor, VdotH);
        vec3 specContrib = intensity * NdotL * BRDF_specularGGX(materialInfo.f0, f90, materialInfo.alphaRoughness, VdotH, NdotL, NdotV, NdotH);

        // Finally, combine the diffuse and specular contributions
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

    if (material.physicalDescriptorTextureID == NOT_PRESENT) {
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

    // Roughness, specular color
    float alphaRoughness = perceptualRoughness * perceptualRoughness;
    vec3 specularColor = mix(f0, baseColor.rgb, metalness);

    // Get the view vector - from surface point to camera
    vec3 v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGlobalPos);

	// Get the normal
    vec3 n = getNormal(material.normalTextureID);

    // Get NdotV
    float NdotV = clamp(abs(dot(n, v)), 0., 1.0);

    vec3 reflection = -normalize(reflect(v, n));

    // Collect all the material info.
    MaterialInfo materialInfo = MaterialInfo(f0, diffuseColor, specularColor, alphaRoughness, perceptualRoughness);

    // Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
    vec3 color;
    if (sceneData.params.x > 0.) {
        color = getIBLContribution(materialInfo, n, reflection, NdotV) * sceneData.params.x;
    } else {
        color = vec3(0.);
    }

    // Apply ambient occlusion, if present.
    if (material.occlusionTextureID != NOT_PRESENT) {
        // Occlusion is stored in the 'g' channel as per:
        // https://github.com/ARM-software/astc-encoder/blob/main/Docs/Encoding.md#encoding-1-4-component-data
        float ao = texture(textures[material.occlusionTextureID], inUV).g;
        color = color * ao;
    }

    // Walk through each light and add its color contribution.
    // Qualcomm's documentation suggests that loops are undesirable, so we do branches instead.
    // Since these values are uniform, they shouldn't have too high of a penalty.
    if (sceneData.lights[0].type != NOT_PRESENT) {
        color += getLightContribution(materialInfo, n, v, NdotV, sceneData.lights[0]);
    }
    if (sceneData.lights[1].type != NOT_PRESENT) {
        color += getLightContribution(materialInfo, n, v, NdotV, sceneData.lights[1]);
    }
    if (sceneData.lights[2].type != NOT_PRESENT) {
        color += getLightContribution(materialInfo, n, v, NdotV, sceneData.lights[2]);
    }
    if (sceneData.lights[3].type != NOT_PRESENT) {
        color += getLightContribution(materialInfo, n, v, NdotV, sceneData.lights[3]);
    }

    // Add emission, if present
    if (material.emissiveTextureID != NOT_PRESENT) {
        vec3 emissive = texture(textures[material.emissiveTextureID], inUV).rgb;
        color += emissive;
    }

    return color;
}
