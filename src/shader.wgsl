// View uniform — identity for now, unblocks future pan/zoom
struct ViewUniform {
    transform: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> view: ViewUniform;


// ── General geometry shader (rectangles, axes, tick marks) ───────────────────

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = view.transform * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}


// ── Shared quad input for instanced pipelines (points and lines) ────────────

struct QuadVertexInput {
    @location(0) offset: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

// Shared fragment output struct for both point and line SDF pipelines
struct SDFVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};


// ── Point shader — SDF anti-aliased instanced circles ───────────────────────

struct PointInstanceInput {
    @location(3) center: vec2<f32>,
    @location(4) half_size: vec2<f32>,
    @location(5) color: vec4<f32>,
};

@vertex
fn vs_point_instanced(
    quad: QuadVertexInput,
    instance: PointInstanceInput,
) -> SDFVertexOutput {
    var out: SDFVertexOutput;
    let world_pos = instance.center + quad.offset * instance.half_size;
    out.clip_position = view.transform * vec4<f32>(world_pos, 0.0, 1.0);
    out.color = instance.color;
    out.uv = quad.uv;
    return out;
}

@fragment
fn fs_point(in: SDFVertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv - vec2<f32>(0.5, 0.5));
    let alpha = 1.0 - smoothstep(0.45, 0.5, dist);
    if alpha < 0.01 {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}


// ── Line shader — miter-join tessellated polylines ──────────────────────────

struct LineVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) edge_dist: f32,
};

struct LineVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) edge_dist: f32,
};

@vertex
fn vs_line(input: LineVertexInput) -> LineVertexOutput {
    var out: LineVertexOutput;
    out.clip_position = view.transform * vec4<f32>(input.position, 0.0, 1.0);
    out.color = input.color;
    out.edge_dist = input.edge_dist;
    return out;
}

@fragment
fn fs_line(in: LineVertexOutput) -> @location(0) vec4<f32> {
    // edge_dist is normalized: 0 at center, ±1.0 at body edge, beyond 1.0 is AA feather.
    // Use fwidth for a 1-pixel anti-aliasing band regardless of line thickness.
    let d = abs(in.edge_dist);
    let aa = fwidth(in.edge_dist) * 0.5;
    let alpha = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);
    if alpha < 0.01 {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
