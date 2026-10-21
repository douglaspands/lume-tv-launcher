//! GLSL ES 1.00 sources. Only OpenGL ES 2.0 features are used.

/// Rounded rectangles from a signed distance function: fills (solid or gradient),
/// rings and soft shadows.
pub const RECT_VERTEX: &str = r#"#version 100
attribute vec4 a_pos_local; // xy: canvas position, zw: position relative to the center
attribute vec4 a_shape;     // xy: half size, z: radius, w: ring width
attribute vec4 a_misc;      // x: softness, yz: gradient axis, w: is gradient
attribute vec4 a_stops;     // xyz: gradient stop positions
attribute vec4 a_c0;
attribute vec4 a_c1;
attribute vec4 a_c2;

uniform vec4 u_view;        // x: scale, yz: offset (device px), w: unused
uniform vec2 u_surface;     // surface size in device px

varying vec3 v_local; // xy: position relative to the center, z: logical units per device pixel
varying vec4 v_shape;
varying vec4 v_misc;
varying vec4 v_stops;
varying vec4 v_c0;
varying vec4 v_c1;
varying vec4 v_c2;

void main() {
    vec2 device = a_pos_local.xy * u_view.x + u_view.yz;
    gl_Position = vec4(device.x / u_surface.x * 2.0 - 1.0, 1.0 - device.y / u_surface.y * 2.0, 0.0, 1.0);
    v_local = vec3(a_pos_local.zw, 1.0 / u_view.x);
    v_shape = a_shape;
    v_misc = a_misc;
    v_stops = a_stops;
    v_c0 = a_c0;
    v_c1 = a_c1;
    v_c2 = a_c2;
}
"#;

pub const RECT_FRAGMENT: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

varying vec3 v_local;
varying vec4 v_shape;
varying vec4 v_misc;
varying vec4 v_stops;
varying vec4 v_c0;
varying vec4 v_c1;
varying vec4 v_c2;

float round_rect_distance(vec2 p, vec2 half_size, float radius) {
    vec2 q = abs(p) - half_size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - radius;
}

void main() {
    float d = round_rect_distance(v_local.xy, v_shape.xy, v_shape.z);
    float unit = v_local.z; // logical units per device pixel
    float alpha;
    if (v_misc.x > 0.0) {
        alpha = 1.0 - smoothstep(-v_misc.x, v_misc.x, d);
    } else {
        alpha = clamp(0.5 - d / unit, 0.0, 1.0);
        if (v_shape.w > 0.0) {
            alpha -= clamp(0.5 - (d + v_shape.w) / unit, 0.0, 1.0);
        }
    }

    vec4 color = v_c0;
    if (v_misc.w > 0.5) {
        float t = clamp(dot(v_local.xy, v_misc.yz) + 0.5, 0.0, 1.0);
        color = mix(v_c0, v_c1, clamp((t - v_stops.x) / max(v_stops.y - v_stops.x, 0.0001), 0.0, 1.0));
        color = mix(color, v_c2, clamp((t - v_stops.y) / max(v_stops.z - v_stops.y, 0.0001), 0.0, 1.0));
    }
    gl_FragColor = color * alpha;
}
"#;

/// Textured quads (glyphs, icons and images). The vertex shader is shared.
pub const TEX_VERTEX: &str = r#"#version 100
attribute vec4 a_pos_uv; // xy: canvas position, zw: texture coordinates
attribute vec4 a_color;
attribute vec4 a_local;  // xy: position relative to center, zw: half size
attribute vec4 a_params; // x: radius

uniform vec4 u_view;
uniform vec2 u_surface;

varying vec2 v_uv;
varying vec4 v_color;
varying vec4 v_shape;    // xy: half size, z: radius, w: 1.0 / u_view.x
varying vec2 v_local;

void main() {
    vec2 device = a_pos_uv.xy * u_view.x + u_view.yz;
    gl_Position = vec4(device.x / u_surface.x * 2.0 - 1.0, 1.0 - device.y / u_surface.y * 2.0, 0.0, 1.0);
    v_uv = a_pos_uv.zw;
    v_color = a_color;
    v_shape = vec4(a_local.zw, a_params.x, 1.0 / u_view.x);
    v_local = a_local.xy;
}
"#;

/// Glyph coverage in the alpha channel of an 8-bit atlas, tinted by the vertex color.
pub const TEXT_FRAGMENT: &str = r#"#version 100
precision mediump float;

uniform sampler2D u_texture;

varying vec2 v_uv;
varying vec4 v_color;

void main() {
    gl_FragColor = v_color * texture2D(u_texture, v_uv).a;
}
"#;

/// Premultiplied RGBA image multiplied by the vertex color, with optional rounded corner clipping.
pub const IMAGE_FRAGMENT: &str = r#"#version 100
precision mediump float;

uniform sampler2D u_texture;

varying vec2 v_uv;
varying vec4 v_color;
varying vec4 v_shape;
varying vec2 v_local;

float round_rect_distance(vec2 p, vec2 half_size, float radius) {
    vec2 q = abs(p) - half_size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - radius;
}

void main() {
    vec4 tex = texture2D(u_texture, v_uv) * v_color;
    if (v_shape.z > 0.0) {
        float d = round_rect_distance(v_local, v_shape.xy, v_shape.z);
        float alpha = clamp(0.5 - d / v_shape.w, 0.0, 1.0);
        tex *= alpha;
    }
    gl_FragColor = tex;
}
"#;
