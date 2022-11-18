float16_t getSquareFalloffAttenuation(float16_t distanceSquare, float16_t falloff) {
    float16_t factor = distanceSquare * falloff;
    float16_t smoothFactor = saturate(F16(1.0) - factor * factor);
    // // We would normally divide by the square distance here
    // // but we do it at the call site
    return smoothFactor * smoothFactor;
}

float16_t getRangeAttenuation(const vec3 posToLight, float16_t falloff) {
    float16_t distanceSquare = F16(dot(posToLight, posToLight));
    float16_t attenuation = getSquareFalloffAttenuation(distanceSquare, falloff);
    // Assume a punctual light occupies a volume of 1cm to avoid a division by 0
    return attenuation / max(distanceSquare, F16(1e-4));
}

// https://github.com/KhronosGroup/glTF/blob/master/extensions/2.0/Khronos/KHR_lights_punctual/README.md#inner-and-outer-cone-angles
float16_t getSpotAttenuation(vec3 l, vec3 spotDirection, float16_t spotScale, float16_t spotOffset) {
    float16_t cd = F16(dot(spotDirection, l));
    float16_t attenuation = saturate(cd * spotScale + spotOffset);
    return attenuation * attenuation;

}

float16_t getLightAttenuation(Light light, vec3 pointToLight, vec3 l) {
    float16_t rangeAttenuation = F16(1.0);
    float16_t spotAttenuation = F16(1.0);

    if (light.type != LightType_Directional) {
        rangeAttenuation = getRangeAttenuation(pointToLight, F16(light.falloff));
    }

    if (light.type == LightType_Spot) {
        spotAttenuation = getSpotAttenuation(l, light.direction, F16(light.lightAngleScale), F16(light.lightAngleOffset));
    }

    return rangeAttenuation * spotAttenuation;
}
