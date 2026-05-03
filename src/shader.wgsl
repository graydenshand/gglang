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

// Per-point fragment input. uv is in [0,1] over the quad; shape_id selects
// the SDF used in fs_point.
struct PointVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) shape_id: u32,
};


// ── Point shader — SDF anti-aliased instanced glyphs ────────────────────────

struct PointInstanceInput {
    @location(3) center: vec2<f32>,
    @location(4) half_size: vec2<f32>,
    @location(5) color: vec4<f32>,
    @location(6) shape_id: u32,
};

@vertex
fn vs_point_instanced(
    quad: QuadVertexInput,
    instance: PointInstanceInput,
) -> PointVertexOutput {
    var out: PointVertexOutput;
    let world_pos = instance.center + quad.offset * instance.half_size;
    out.clip_position = view.transform * vec4<f32>(world_pos, 0.0, 1.0);
    out.color = instance.color;
    out.uv = quad.uv;
    out.shape_id = instance.shape_id;
    return out;
}

// Equilateral triangle SDF (apex up), normalized to fit within unit box.
fn sd_triangle(p: vec2<f32>) -> f32 {
    let k = sqrt(3.0);
    var q = vec2<f32>(abs(p.x) - 0.5, p.y + 0.5 / k);
    if (q.x + k * q.y > 0.0) {
        q = vec2<f32>(q.x - k * q.y, -k * q.x - q.y) / 2.0;
    }
    q.x = q.x - clamp(q.x, -1.0, 0.0);
    return -length(q) * sign(q.y);
}

fn sd_box(p: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = abs(p) - b;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

@fragment
fn fs_point(in: PointVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv - vec2<f32>(0.5, 0.5);
    var dist: f32 = 0.0;
    var threshold: f32 = 0.5;
    // All shapes are inscribed in the radius-0.5 disk. Per-shape pixel
    // multipliers in PointInstance build (`shape_size_multiplier`) scale the
    // quad up so each glyph ends up with roughly the same screen area.
    if (in.shape_id == 0u) {
        dist = length(p);
        threshold = 0.5;
    } else if (in.shape_id == 1u) {
        // Equilateral triangle inscribed in radius-0.5 disk: side ≈ 0.866.
        dist = sd_triangle(p * 1.1547);
        threshold = 0.0;
    } else if (in.shape_id == 2u) {
        // Square inscribed in radius-0.5 disk: half-side = 1/(2·sqrt(2)).
        dist = max(abs(p.x), abs(p.y));
        threshold = 0.3536;
    } else if (in.shape_id == 3u) {
        dist = abs(p.x) + abs(p.y);
        threshold = 0.5;
    } else {
        // Cross / plus: union of two thin boxes spanning the full disk.
        let arm = vec2<f32>(0.5, 0.15);
        dist = min(sd_box(p, arm), sd_box(p, arm.yx));
        threshold = 0.0;
    }
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(threshold - aa, threshold + aa, dist);
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
