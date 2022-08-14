// Adapted from https://github.com/MatchaChoco010/egui-winit-ash-integration/blob/main/src/shaders/src/vert.vert
#version 450

layout (location = 0) in vec2 inPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) in vec4 inColor;

layout (location = 0) out vec4 outColor;
layout (location = 1) out vec2 outUV;

layout (push_constant) uniform PushConstants {
    vec2 screen_size;
} pushConstants;

vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 lower = srgb / vec3(12.92);
    vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    return mix(higher, lower, cutoff);
}

void main() {
    gl_Position = vec4(
        2.0 * inPos.x / pushConstants.screen_size.x - 1.0,
        2.0 * inPos.y / pushConstants.screen_size.y - 1.0,
        0.0,
        1.0);
    outColor = vec4(srgb_to_linear(inColor.rgb), inColor.a);
    outUV = inUV;
}
