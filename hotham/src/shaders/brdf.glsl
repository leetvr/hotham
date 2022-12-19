// Filament's lambertian term is 1.0 / PI, so we just pre-compute that
#define BRDF_LAMBERTIAN F16(0.3183098862)

// The following equation models the Fresnel reflectance term of the spec equation (aka F())
// Implementation of fresnel from [4], Equation 15

// Anything less than 2% is physically impossible and is instead considered to be shadowing. Compare to "Real-Time-Rendering" 4th edition on page 325.
const f16vec3 f90 = V16(1.0);

// f16vec3 F_Schlick(f16vec3 f0, float16_tVdotH) {
//     return f0 + (f90 - f0) * pow(clamp(1.0 - VdotH, 0.0, 1.0), 5.0);
// }

// Smith Joint GGX
// Note: Vis = G / (4 * NdotL * NdotV)
// see Eric Heitz. 2014. Understanding the Masking-Shadowing Function in Microfacet-Based BRDFs. Journal of Computer Graphics Techniques, 3
// see Real-Time Rendering. Page 331 to 336.
// see https://google.github.io/filament/Filament.md.html#materialsystem/specularbrdf/geometricshadowing(specularg)
// float16_t V_GGX(float16_t NdotL, float16_t NdotV, float16_t alphaRoughness) {
//     float16_t alphaRoughnessSq = alphaRoughness * alphaRoughness;

//     float16_t GGXV = NdotL * sqrt(NdotV * NdotV * (1.0 - alphaRoughnessSq) + alphaRoughnessSq);
//     float16_t GGXL = NdotV * sqrt(NdotL * NdotL * (1.0 - alphaRoughnessSq) + alphaRoughnessSq);

//     float16_t GGX = GGXV + GGXL;
//     if (GGX > 0.0) {
//         return 0.5 / GGX;
//     }
//     return 0.0;
// }

float16_t D_GGX(float16_t roughness, f16vec3 n, float16_t NdotH, f16vec3 h) {
    f16vec3 NxH = cross(n, h);
    float16_t oneMinusNoHSquared = dot(NxH, NxH);

    float16_t a = NdotH * roughness;
    float16_t k = roughness / (oneMinusNoHSquared + a * a);
    float16_t d = k * k * BRDF_LAMBERTIAN;
    return saturateMediump(d);
}

float16_t V_SmithGGXCorrelated_Fast(float16_t roughness, float16_t NdotV, float16_t NdotL) {
    // Hammon 2017, "PBR Diffuse Lighting for GGX+Smith Microsurfaces"
    float16_t v = F16(0.5) / mix(F16(2.0) * NdotL * NdotV, NdotL + NdotV, roughness);
    return saturateMediump(v);
}

// KR: why is this VdotH instead of LdotH?
f16vec3 F_Schlick(const f16vec3 f0, float16_t VdotH) {
    float16_t f = pow(F16(1.0) - VdotH, F16(5.0));
    return f + f0 * (F16(1.0) - f);
}

// //  https://github.com/KhronosGroup/glTF/tree/master/specification/2.0#acknowledgments AppendixB
// f16vec3 BRDF_specularGGX(f16vec3 f0, float16_talphaRoughness, float16_tVdotH, float16_tNdotL, float16_tNdotV, float16_tNdotH) {
//     f16vec3 F = F_Schlick(f0, VdotH);
//     float16_tVis = V_GGX(NdotL, NdotV, alphaRoughness);
//     float16_tD = D_GGX(NdotH, alphaRoughness);

//     return F * Vis * D;
// }

f16vec3 BRDF_specular(const f16vec3 f0, const float16_t roughness, f16vec3 h, const f16vec3 n, float16_t NdotV, float16_t NdotL, float16_t NdotH, float16_t LdotH) {
    float16_t D = D_GGX(roughness, n, NdotH, h);
    float16_t V = V_SmithGGXCorrelated_Fast(roughness, NdotV, NdotL);
    f16vec3   F = F_Schlick(f0, LdotH);

    return (D * V) * F;
}
