#version 440 core

#include <constants.glsl>

#define LIGHTING_TYPE LIGHTING_TYPE_REFLECTION

#define LIGHTING_REFLECTION_KIND LIGHTING_REFLECTION_KIND_GLOSSY

#if (FLUID_MODE == FLUID_MODE_LOW)
    #define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_IMPORTANCE
#elif (FLUID_MODE >= FLUID_MODE_MEDIUM)
    #define LIGHTING_TRANSPORT_MODE LIGHTING_TRANSPORT_MODE_RADIANCE
#endif

#define LIGHTING_DISTRIBUTION_SCHEME LIGHTING_DISTRIBUTION_SCHEME_MICROFACET

#define LIGHTING_DISTRIBUTION LIGHTING_DISTRIBUTION_BECKMANN

#define HAS_SHADOW_MAPS

#include <globals.glsl>

layout(location = 0) in vec3 f_pos;
layout(location = 0) out vec4 tgt_color;

#include <sky.glsl>
#include <light.glsl>
#include <lod.glsl>

const float FADE_DIST = 32.0;

void main() {
    float dist = length(f_pos - cam_pos.xyz);
    float dist_fade = 1.0 - clamp(dist / FADE_DIST, 0.0, 1.0);

    // Warm near the player, cool at the trail's fading edge.
    vec3 near_color = vec3(1.0, 0.84, 0.55);
    vec3 far_color = vec3(0.35, 0.72, 1.0);
    vec3 trail_color = mix(far_color, near_color, dist_fade);

    // Fade smoothly with distance while keeping trails readable in daylight.
    float trail_alpha = 0.08 * dist_fade;
    trail_alpha += get_sun_brightness() * 0.03;

    tgt_color = vec4(trail_color, trail_alpha);
}
