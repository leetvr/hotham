#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : require

layout(binding = 1) uniform sampler2D textureSampler;
layout(binding = 2) uniform sampler2D normalSampler;

layout(location = 0) in vec2 inTextureCoordinates;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec3 inViewVec;
layout(location = 3) in vec3 inLightVec;
layout(location = 4) in vec4 inTangent;

layout(location = 0) out vec4 outFragColor;

void main() {
    vec4 color = texture(textureSampler, inTextureCoordinates);

    vec3 N = normalize(inNormal);
	vec3 T = normalize(inTangent.xyz);
	vec3 B = cross(inNormal, inTangent.xyz) * inTangent.w;
	mat3 TBN = mat3(T, B, N);
	N = TBN * normalize(texture(normalSampler, inTextureCoordinates).xyz * 2.0 - vec3(1.0));

    const float ambient = 0.05;
	vec3 L = normalize(inLightVec);
	vec3 V = normalize(inViewVec);
	vec3 R = reflect(-L, N);
	vec3 diffuse = max(dot(N, L), ambient).rrr;
	float specular = pow(max(dot(R, V), 0.0), 32.0);
	// outFragColor = vec4(diffuse * color.rgb + specular, color.a);
	outFragColor = vec4(1.0);
}