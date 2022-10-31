// Fresnel
//
// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// https://github.com/wdas/brdf/tree/master/src/brdfs
// https://google.github.io/filament/Filament.md.html
//

// The following equation models the Fresnel reflectance term of the spec equation (aka F())
// Implementation of fresnel from [4], Equation 15
const float16_t M_PI_F16 = float16_t(3.141592653589793);
const float M_PI_F32 = float(3.141592653589793);

// Anything less than 2% is physically impossible and is instead considered to be shadowing. Compare to "Real-Time-Rendering" 4th edition on page 325.
const f16vec3 f90 = f16vec3(1.0);

f16vec3 F_Schlick(f16vec3 f0, float16_t VdotH) {
    return f0 + (f90 - f0) * pow(clamp(float16_t(1.0) - VdotH, float16_t(0.0), float16_t(1.0)), float16_t(5.0));
}

// Smith Joint GGX
// Note: Vis = G / (4 * NdotL * NdotV)
// see Eric Heitz. 2014. Understanding the Masking-Shadowing Function in Microfacet-Based BRDFs. Journal of Computer Graphics Techniques, 3
// see Real-Time Rendering. Page 331 to 336.
// see https://google.github.io/filament/Filament.md.html#materialsystem/specularbrdf/geometricshadowing(specularg)
float16_t V_GGX(float16_t NdotL, float16_t NdotV, float16_t alphaRoughness) {
    float16_t alphaRoughnessSq = alphaRoughness * alphaRoughness;

    float16_t GGXV = NdotL * sqrt(NdotV * NdotV * (float16_t(1.0) - alphaRoughnessSq) + alphaRoughnessSq);
    float16_t GGXL = NdotV * sqrt(NdotL * NdotL * (float16_t(1.0) - alphaRoughnessSq) + alphaRoughnessSq);

    float16_t GGX = GGXV + GGXL;
    if (GGX > 0.0) {
        return float16_t(0.5) / GGX;
    }
    return float16_t(0.0);
}

// The following equation(s) model the distribution of microfacet normals across the area being drawn (aka D())
// Implementation from "Average Irregularity Representation of a Roughened Surface for Ray Reflection" by T. S. Trowbridge, and K. P. Reitz
// Follows the distribution function recommended in the SIGGRAPH 2013 course notes from EPIC Games [1], Equation 3.
float16_t D_GGX(float16_t NdotH, float16_t alphaRoughness) {
    const float alphaRoughnessSq = float(alphaRoughness) * float(alphaRoughness);
    // f gets poor precision with float16 if alphaRoughnessSq is too small.
    const float f = float(NdotH) * float(NdotH) * (alphaRoughnessSq - 1.0) + 1.0;
    return float16_t(alphaRoughnessSq / (M_PI_F32 * f * f));
}

//https://github.com/KhronosGroup/glTF/tree/master/specification/2.0#acknowledgments AppendixB
f16vec3 BRDF_lambertian(f16vec3 f0, f16vec3 diffuseColor, float16_t VdotH) {
    // see https://seblagarde.wordpress.com/2012/01/08/pi-or-not-to-pi-in-game-lighting-equation/
    return (float16_t(1.0) - F_Schlick(f0, VdotH)) * (diffuseColor / M_PI_F16);
}

//  https://github.com/KhronosGroup/glTF/tree/master/specification/2.0#acknowledgments AppendixB
f16vec3 BRDF_specularGGX(f16vec3 f0, float16_t alphaRoughness, float16_t VdotH, float16_t NdotL, float16_t NdotV, float16_t NdotH) {
    f16vec3 F = F_Schlick(f0, VdotH);
    float16_t Vis = V_GGX(NdotL, NdotV, alphaRoughness);
    float16_t D = D_GGX(NdotH, alphaRoughness);

    return F * Vis * D;
}
