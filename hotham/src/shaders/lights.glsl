// Light related functionality, mostly borrowed from:
// https://github.com/KhronosGroup/glTF-Sample-Viewer/blob/master/source/Renderer/shaders/punctual.glsl

// https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_lights_punctual/README.md#range-property
float16_t getRangeAttenuation(float16_t range, float16_t distance) {
    if (range <= 0.0) {
        // negative range means unlimited
        return float16_t(1.0) / pow(distance, float16_t(2.0));
    }
    return max(min(float16_t(1.0) - pow(distance / range, float16_t(4.0)), float16_t(1.0)), float16_t(0.0)) / pow(distance, float16_t(2.0));
}

// https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_lights_punctual/README.md#inner-and-outer-cone-angles
float16_t getSpotAttenuation(f16vec3 pointToLight, f16vec3 spotDirection, float16_t outerConeCos, float16_t innerConeCos) {
    float16_t actualCos = dot(normalize(spotDirection), normalize(-pointToLight));
    if (actualCos > outerConeCos) {
        if (actualCos < innerConeCos) {
            return smoothstep(outerConeCos, innerConeCos, actualCos);
        }
        return float16_t(1.0);
    }
    return float16_t(0.0);
}

f16vec3 getLightIntensity(Light light, f16vec3 pointToLight) {
    float16_t rangeAttenuation = float16_t(1.0);
    float16_t spotAttenuation = float16_t(1.0);

    if (light.type != LightType_Directional) {
        rangeAttenuation = getRangeAttenuation(float16_t(light.range), length(pointToLight));
    }

    if (light.type == LightType_Spot) {
        spotAttenuation = getSpotAttenuation(pointToLight, f16vec3(light.direction), float16_t(light.outerConeCos), float16_t(light.innerConeCos));
    }

    return rangeAttenuation * spotAttenuation * float16_t(light.intensity) * f16vec3(light.color);
}
