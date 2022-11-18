// Light related functionality, mostly borrowed from:
// https://github.com/KhronosGroup/glTF-Sample-Viewer/blob/master/source/Renderer/shaders/punctual.glsl

// https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_lights_punctual/README.md#range-property
float16_t getRangeAttenuation(float16_t range, float16_t distance) {
    if (range <= 0.0) {
        // negative range means unlimited
        return F16(1.0) / pow(distance, F16(2.0));
    }
    return max(min(F16(1.0) - pow(distance / range, F16(4.0)), F16(1.0)), F16(0.0) / pow(distance, F16(2.0)));
}

// https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_lights_punctual/README.md#inner-and-outer-cone-angles
float16_t getSpotAttenuation(f16vec3 pointToLight, f16vec3 spotDirection, float16_t outerConeCos, float16_t innerConeCos) {
    float16_t actualCos = dot(normalize(spotDirection), normalize(-pointToLight));
    if (actualCos > outerConeCos) {
        if (actualCos < innerConeCos) {
            return smoothstep(outerConeCos, innerConeCos, actualCos);
        }
        return F16(1);
    }
    return F16(0);
}

f16vec3 getLightIntensity(Light light, f16vec3 pointToLight) {
    float16_t rangeAttenuation = F16(1.0);
    float16_t spotAttenuation = F16(1.0);

    if (light.type != LightType_Directional) {
        rangeAttenuation = getRangeAttenuation(F16(light.range), length(pointToLight));
    }

    if (light.type == LightType_Spot) {
        spotAttenuation = getSpotAttenuation(pointToLight, V16(light.direction), F16(light.outerConeCos), F16(light.innerConeCos));
    }

    return rangeAttenuation * spotAttenuation * V16(light.intensity) * V16(light.color);
}
