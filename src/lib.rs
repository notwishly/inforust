use js_sys::Float32Array;
use wasm_bindgen::prelude::*;

/// Adaptive visual quality: 0 = full, 1 = balanced, 2 = low-power.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct DeviceProfile {
    level: u8,
    fps_cap: u8,
    dpr_cap: f32,
    snow_scale: f32,
    blur_scale: f32,
    glow_enabled: bool,
    lens_enabled: bool,
}

#[wasm_bindgen]
impl DeviceProfile {
    #[wasm_bindgen(constructor)]
    pub fn new(cores: u32, memory_gb: f32, save_data: bool, reduced_motion: bool) -> DeviceProfile {
        let weak_cpu = cores > 0 && cores <= 2;
        let low_memory = memory_gb > 0.0 && memory_gb <= 2.0;
        let constrained = (cores > 0 && cores <= 4) || (memory_gb > 0.0 && memory_gb <= 4.0) || save_data;
        let level = if weak_cpu || low_memory { 2 } else if constrained { 1 } else { 0 };
        let mut p = match level {
            2 => DeviceProfile { level, fps_cap: 30, dpr_cap: 1.0, snow_scale: 0.6, blur_scale: 0.45, glow_enabled: false, lens_enabled: false },
            1 => DeviceProfile { level, fps_cap: 45, dpr_cap: 1.25, snow_scale: 0.65, blur_scale: 0.7, glow_enabled: true, lens_enabled: false },
            _ => DeviceProfile { level, fps_cap: 60, dpr_cap: 2.0, snow_scale: 1.0, blur_scale: 1.0, glow_enabled: true, lens_enabled: true },
        };
        // Reduced motion changes animation behavior, not visual quality.
        if reduced_motion { p.fps_cap = 1; }
        p
    }
    pub fn level(&self) -> u8 { self.level }
    pub fn fps_cap(&self) -> u8 { self.fps_cap }
    pub fn dpr_cap(&self) -> f32 { self.dpr_cap }
    pub fn snow_scale(&self) -> f32 { self.snow_scale }
    pub fn blur_scale(&self) -> f32 { self.blur_scale }
    pub fn glow_enabled(&self) -> bool { self.glow_enabled }
    pub fn lens_enabled(&self) -> bool { self.lens_enabled }
}

#[derive(Clone, Copy)]
struct Clump { x: f32, rx: f32, ry: f32, k: f32, g: f32, dying: bool, mt: f64, ph: f32, wf: f32 }
struct Surface { width: f32, max_h: f32, radius: f32, clumps: Vec<Clump>, rest: f32, timer: f32, filled: bool, rng: u32 }
#[derive(Clone, Copy)]
struct Drop { x: f32, y: f32, vx: f32, vy: f32, r: f32, life: f32 }
impl Surface {
    fn random(&mut self) -> f32 {
        // Small deterministic xorshift PRNG; avoids crossing the JS/WASM boundary per particle.
        let mut x = self.rng.max(1); x ^= x << 13; x ^= x >> 17; x ^= x << 5; self.rng = x;
        (x as f64 / u32::MAX as f64) as f32
    }
    fn margin(&self) -> f32 { self.radius * 0.85 + 2.0 }
    fn target(&self, width: f32, scale: f32) -> usize {
        let base = (width / (self.max_h * 1.3).max(1.0) * 1.6).round() + 3.0;
        (base * scale).round().max(0.0) as usize
    }
    fn profile(c: &Clump, u: f32) -> f32 {
        let b = (1.0 + (u * std::f32::consts::PI).cos()) * 0.5;
        b.powf(0.72) * (1.0 + c.wf * (u * 7.0 + c.ph).sin() * (1.0 - u.abs()))
    }
    fn height_at(&self, x: f32) -> f32 {
        self.clumps.iter().fold(0.0_f32, |h, c| {
            let u = (x - c.x) / c.rx.max(0.001);
            if u * u < 1.0 { h.max(c.ry * c.k * Self::profile(c, u)) } else { h }
        })
    }
    fn spawn(&mut self, scale: f32) {
        let m = self.margin(); let lo = m; let hi = self.width - m;
        if hi - lo < 8.0 { return; }
        let mut x = lo; let mut h0 = 0.0;
        for _ in 0..4 {
            if !self.clumps.is_empty() && self.random() < 0.55 {
                let idx = (self.random() * self.clumps.len() as f32) as usize;
                let c = self.clumps[idx.min(self.clumps.len()-1)];
                x = c.x + (self.random() * 2.0 - 1.0) * c.rx * 0.9;
            } else { x = lo + self.random() * (hi - lo); }
            h0 = self.height_at(x);
            if h0 <= self.max_h * 0.9 { break; }
        }
        let ry = self.max_h.min((self.max_h * (0.35 + self.random() * 0.4)).max(h0 * (1.1 + self.random() * 0.3)));
        let rx = (ry * (2.1 + self.random() * 1.3)).min((hi - lo) * 0.5);
        let c = Clump { x: x.max(lo + rx).min(hi - rx), rx, ry, k: scale, g: 1.0 / (0.8 + self.random() * 0.8), dying: false, mt: 0.0, ph: self.random() * 6.28, wf: 0.02 + self.random() * 0.035 };
        self.clumps.push(c);
    }
}

fn step_surface(s: &mut Surface, drops: &mut Vec<Drop>, dt: f32, now_ms: f64, visible: bool, speed: f32, scale: f32, reduced_motion: bool, screen_x: f32, screen_y: f32) -> bool {
    let mut dirty = false;
    if reduced_motion {
        if !s.filled { s.filled = true; let n = s.target(s.width, scale); for _ in 0..n { s.spawn(1.0); } dirty = true; }
        return dirty;
    }
    s.rest = (s.rest - dt).max(0.0);
    let mut i = s.clumps.len();
    while i > 0 {
        i -= 1;
        let dying = s.clumps[i].dying;
        if dying {
            if now_ms < s.clumps[i].mt { continue; }
            s.clumps[i].k -= dt * 2.8;
            if visible { dirty = true; }
            let k = s.clumps[i].k;
            if visible && k > 0.25 && drops.len() < 90 && s.random() < 0.02 {
                let c = s.clumps[i];
                let x = screen_x + c.x + (s.random() * 2.0 - 1.0) * c.rx * 0.7;
                let y = screen_y + 1.2 - c.ry * c.k * 0.3;
                let vx = (s.random() - 0.5) * 12.0;
                let vy = 10.0 + s.random() * 30.0;
                let r = 0.8 + s.random() * 1.3;
                drops.push(Drop { x, y, vx, vy, r, life: 1.0 });
            }
            if s.clumps[i].k <= 0.0 { s.clumps.remove(i); }
        } else if s.clumps[i].k < 1.0 {
            s.clumps[i].k = (s.clumps[i].k + dt * s.clumps[i].g).min(1.0);
            if visible { dirty = true; }
        }
    }
    if s.rest <= 0.0 && s.clumps.len() < s.target(s.width, scale) {
        s.timer -= dt;
        if s.timer <= 0.0 {
            s.spawn(0.0);
            s.timer = (0.35 + s.random() * 0.7) / speed.max(0.1) * (0.45 + 0.55 * s.max_h / 20.0);
            if visible { dirty = true; }
        }
    }
    dirty
}

/// Owns snow accumulation, growth and melting state. Canvas drawing remains in JS.
#[wasm_bindgen]
pub struct SnowEngine { surfaces: Vec<Surface>, drops: Vec<Drop>, frame_buffer: Vec<f32> }
#[wasm_bindgen]
impl SnowEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SnowEngine { SnowEngine { surfaces: Vec::new(), drops: Vec::new(), frame_buffer: Vec::new() } }
    pub fn add_surface(&mut self, width: f32, max_h: f32, radius: f32, seed: u32) -> u32 {
        let id = self.surfaces.len() as u32;
        self.surfaces.push(Surface { width, max_h, radius, clumps: Vec::new(), rest: 0.0, timer: 0.2, filled: false, rng: seed.max(1) }); id
    }
    pub fn resize_surface(&mut self, id: u32, width: f32) {
        if let Some(s) = self.surfaces.get_mut(id as usize) {
            s.width = width; let m = s.margin(); s.clumps.retain(|c| c.x - c.rx >= m - 1.0 && c.x + c.rx <= width - m + 1.0);
        }
    }
    pub fn step(&mut self, id: u32, dt: f32, now_ms: f64, visible: bool, speed: f32, scale: f32, reduced_motion: bool, screen_x: f32, screen_y: f32) -> bool {
        let SnowEngine { surfaces, drops, .. } = self;
        let Some(s) = surfaces.get_mut(id as usize) else { return false; };
        step_surface(s, drops, dt, now_ms, visible, speed, scale, reduced_motion, screen_x, screen_y)
    }
    /// Update every surface in one JS/WASM call. Each descriptor is [id, visible, screen_x, screen_y].
    pub fn step_all(&mut self, frames: &Float32Array, dt: f32, now_ms: f64, speed: f32, scale: f32, reduced_motion: bool) -> bool {
        let SnowEngine { surfaces, drops, frame_buffer } = self;
        frame_buffer.resize(frames.length() as usize, 0.0);
        frames.copy_to(frame_buffer);
        let mut dirty = false;
        for row in frame_buffer.chunks_exact(4) {
            if row[1] < -0.5 { continue; }
            let id = row[0] as usize;
            if let Some(s) = surfaces.get_mut(id) {
                dirty |= step_surface(s, drops, dt, now_ms, row[1] > 0.5, speed, scale, reduced_motion, row[2], row[3]);
            }
        }
        dirty
    }
    pub fn melt(&mut self, id: u32, x: f32, now_ms: f64) {
        if let Some(s) = self.surfaces.get_mut(id as usize) {
            for c in &mut s.clumps { c.dying = true; c.mt = now_ms + ((c.x - x).abs() / 700.0 * 1000.0) as f64; }
            s.rest = 2.0 + s.width / 700.0;
        }
    }
    /// Advance water drops and return packed rows: x, y, radius, remaining life.
    /// Combining update + export avoids multiple JS/WASM crossings per frame.
    pub fn advance_drops(&mut self, dt: f32) -> JsValue {
        for d in &mut self.drops { d.vy += 260.0 * dt; d.x += d.vx * dt; d.y += d.vy * dt; d.life -= dt * 1.6; }
        self.drops.retain(|d| d.life > 0.0);
        if self.drops.is_empty() { return JsValue::UNDEFINED; }
        let mut data = Vec::with_capacity(self.drops.len() * 4);
        for d in &self.drops { data.extend_from_slice(&[d.x, d.y, d.r, d.life]); }
        Float32Array::from(data.as_slice()).into()
    }
    /// Immediately trim existing snow when runtime quality steps down.
    pub fn set_density(&mut self, scale: f32) {
        for s in &mut self.surfaces {
            let target = s.target(s.width, scale);
            if s.clumps.len() > target { s.clumps.truncate(target); }
        }
    }
    pub fn clear_surface(&mut self, id: u32) { if let Some(s) = self.surfaces.get_mut(id as usize) { s.clumps.clear(); s.rest = 0.0; } }
    pub fn clump_count(&self, id: u32) -> usize { self.surfaces.get(id as usize).map_or(0, |s| s.clumps.len()) }
    /// Packed path vertices: 21 (x, height) pairs per clump.
    /// Trigonometry and snow contour generation stay inside Rust/WASM.
    pub fn paths(&self, id: u32) -> Float32Array {
        let Some(s) = self.surfaces.get(id as usize) else { return Float32Array::new_with_length(0); };
        let mut data = Vec::with_capacity(s.clumps.len() * 42);
        for c in &s.clumps {
            let eased = 1.0 - (1.0 - c.k.min(1.0)).powi(3);
            if eased <= 0.01 { continue; }
            for i in 0..=20 {
                let u = -1.0 + 2.0 * i as f32 / 20.0;
                let height = if i == 0 || i == 20 { 0.0 } else { c.ry * eased * Surface::profile(c, u) };
                data.extend_from_slice(&[c.x + u * c.rx, height]);
            }
        }
        Float32Array::from(data.as_slice())
    }
    /// Packed rows: x, radius-x, radius-y, growth, phase, wave-factor.
    pub fn clumps(&self, id: u32) -> Float32Array {
        let Some(s) = self.surfaces.get(id as usize) else { return Float32Array::new_with_length(0); };
        let mut data = Vec::with_capacity(s.clumps.len() * 6);
        for c in &s.clumps { data.extend_from_slice(&[c.x, c.rx, c.ry, c.k, c.ph, c.wf]); }
        Float32Array::from(data.as_slice())
    }
}

#[wasm_bindgen]
pub fn benchmark(iterations: u32) -> f64 {
    let mut x = 0.3141592653589793_f64;
    for i in 0..iterations.min(50_000_000) { x = (x + (i as f64 * 0.000001)).sin().abs(); x = (x * 1.000000119 + 0.0000001).sqrt(); }
    x
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn creates_and_advances_snow() {
        let mut e = SnowEngine::new(); let id = e.add_surface(320.0, 12.0, 8.0, 123);
        for i in 0..120 { e.step(id, 1.0/30.0, i as f64*33.3, true, 1.0, 1.0, false, 0.0, 0.0); }
        assert!(e.clump_count(id) > 0);
    }
    #[test] fn target_matches_original_javascript_rounding() {
        let s = Surface { width: 320.0, max_h: 12.0, radius: 8.0, clumps: Vec::new(), rest: 0.0, timer: 0.2, filled: false, rng: 1 };
        assert_eq!(s.target(320.0, 1.0), 36);
        assert_eq!(s.target(320.0, 0.6), 22);
    }
    #[test] fn low_profile_caps_quality_without_disabling_reduced_motion_visuals() {
        let low = DeviceProfile::new(2, 2.0, false, false);
        assert_eq!(low.level(), 2);
        assert_eq!(low.fps_cap(), 30);
        assert_eq!(low.dpr_cap(), 1.0);
        assert_eq!(low.snow_scale(), 0.6);
        assert!(!low.glow_enabled());
        assert!(!low.lens_enabled());

        let reduced = DeviceProfile::new(8, 8.0, false, true);
        assert_eq!(reduced.level(), 0);
        assert_eq!(reduced.fps_cap(), 1);
    }
    #[test] fn density_step_down_trims_existing_clumps() {
        let mut e = SnowEngine::new();
        let id = e.add_surface(320.0, 12.0, 8.0, 7);
        let surface = &mut e.surfaces[id as usize];
        for i in 0..30 {
            surface.clumps.push(Clump { x: i as f32, rx: 2.0, ry: 2.0, k: 1.0, g: 1.0, dying: false, mt: 0.0, ph: 0.0, wf: 0.02 });
        }
        e.set_density(0.6);
        assert_eq!(e.clump_count(id), 22);
    }
    #[test] fn reduced_motion_fills_surface_once() {
        let mut s = Surface { width: 320.0, max_h: 12.0, radius: 8.0, clumps: Vec::new(), rest: 0.0, timer: 0.2, filled: false, rng: 123 };
        let mut drops = Vec::new();
        step_surface(&mut s, &mut drops, 1.0 / 60.0, 0.0, true, 1.0, 0.6, true, 0.0, 0.0);
        let count = s.clumps.len();
        assert!(count > 0);
        step_surface(&mut s, &mut drops, 1.0 / 60.0, 1000.0, true, 1.0, 0.6, true, 0.0, 0.0);
        assert_eq!(s.clumps.len(), count);
        assert!(drops.is_empty());
    }
}
