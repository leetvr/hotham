#version 450
#extension GL_EXT_multiview : require

layout (binding = 0) uniform sampler2DArray samplerView;

layout (location = 0) in vec2 inUV;
layout (location = 0) out vec4 outColor;

layout (constant_id = 0) const float VIEW_LAYER = 0.0f;

void main() 
{
	const float alpha = 0.0;

	vec2 p1 = vec2(2.0 * inUV - 1.0);
	vec2 p2 = p1 / (1.0 - alpha * length(p1));
	p2 = (p2 + 1.0) * 0.5;

	bool inside = ((p2.x >= 0.0) && (p2.x <= 1.0) && (p2.y >= 0.0 ) && (p2.y <= 1.0));
	outColor = inside ? texture(samplerView, vec3(p2, VIEW_LAYER)) : vec4(0.0);
}