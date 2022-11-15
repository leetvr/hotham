#define DEFAULT_EXPOSURE 1.0
#define DEFAULT_IBL_SCALE 0.4
#define DEFAULT_CUBE_MIPMAP_LEVELS 10
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1
#define ERROR_MAGENTA vec4(1., 0., 1., 1.)

#define TEXTURE_FLAG_HAS_PBR_TEXTURES 1
#define TEXTURE_FLAG_HAS_NORMAL_MAP 2
#define TEXTURE_FLAG_HAS_AO_TEXTURE 4
#define TEXTURE_FLAG_HAS_EMISSION_TEXTURE 8

struct Material {
    uint textureFlags;
    uint baseTextureID;
};

// Store the material in a global to avoid copying when calling functions.
Material material;

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
    if ((material.textureFlags & TEXTURE_FLAG_HAS_NORMAL_MAP) == 0) {
        return N;
    }

    vec3 textureNormal;
    textureNormal.xy = texture(textures[material.baseTextureID + 2], inUV).ga * 2.0 - 1.0;
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

vec3 getPBRMetallicRoughnessColor(vec4 baseColor) {

    // Metallic and Roughness material properties are packed together
    // In glTF, these factors can be specified by fixed scalar values
    // or from a metallic-roughness map
    float perceptualRoughness = 1.0;
    float metalness = 1.0;

    if ((material.textureFlags & TEXTURE_FLAG_HAS_PBR_TEXTURES) == 0) {
        perceptualRoughness = clamp(perceptualRoughness, 0., 1.0);
        metalness = clamp(metalness, 0., 1.0);
    } else {
        // As per the glTF spec:
        // The textures for metalness and roughness properties are packed together in a single texture called metallicRoughnessTexture.
        // Its green channel contains roughness values and its blue channel contains metalness values.
        vec4 mrSample = texture(textures[material.baseTextureID + 1], inUV);

        perceptualRoughness = clamp(mrSample.g * perceptualRoughness, 0.0, 1.0);
        metalness = clamp(mrSample.b * metalness, 0.0, 1.0);
    }

    // Get this material's f0
    vec3 f0 = mix(vec3(DEFAULT_F0), baseColor.rgb, metalness);

    // Get the diffuse color
    vec3 diffuseColor = mix(baseColor.rgb, vec3(0.), metalness);

    // Roughness is authored as perceptual roughness; as is convention,
    // convert to material roughness by squaring the perceptual roughness
    float alphaRoughness = perceptualRoughness * perceptualRoughness;

    // Get the view vector - from surface point to camera
    vec3 v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGosPos);

    // Get the normal
    vec3 n = getNormal();

    // Get NdotV and reflection
    float NdotV = clamp(abs(dot(n, v)), 0., 1.0);
    vec3 reflection = normalize(reflect(-v, n));

    // Calculate lighting contribution from image based lighting source (IBL), scaled by a scene data parameter.
    vec3 color;
    if (sceneData.params.x > 0.) {
        color = getIBLContribution(f0, perceptualRoughness, diffuseColor, reflection, NdotV) * sceneData.params.x;
    } else {
        color = vec3(0.);
    }

    // Apply ambient occlusion, if present.
    if ((material.textureFlags & TEXTURE_FLAG_HAS_AO_TEXTURE) != 0) {
        // Occlusion is stored in the 'r' channel as per the glTF spec
        float ao = texture(textures[material.baseTextureID + 1], inUV).r;
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
    if ((material.textureFlags & TEXTURE_FLAG_HAS_EMISSION_TEXTURE) != 0) {
        color += texture(textures[material.baseTextureID + 3], inUV).rgb;
    }
    return color;
}
