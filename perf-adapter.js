/*
 * Browser-side adapter: keeps DOM, CSS animations, Discord, audio and APIs in
 * their native browser environment. Rust/WASM chooses a conservative profile.
 *
 * The adapter stays idle unless the Rust/WASM runtime initializes successfully.
 */
import { startRustEngine } from "./wasm-bridge.js";

const clamp = (n, lo, hi) => Math.max(lo, Math.min(hi, n));

async function boot() {
  const engine = await startRustEngine();
  if (!engine) return;

  const profile = engine.profile;
  const targetFps = profile.fps_cap();
  let frames = 0;
  let start = performance.now();
  let lowWindows = 0;

  // Runtime guard: if real frame delivery stays below target, step down once.
  // This does not use a permanent animation loop when the tab is backgrounded.
  function sample(now) {
    if (document.hidden) {
      requestAnimationFrame(sample);
      return;
    }
    frames++;
    if (now - start >= 2500) {
      const fps = frames * 1000 / (now - start);
      if (fps < targetFps * 0.72) lowWindows++;
      else lowWindows = Math.max(0, lowWindows - 1);

      if (lowWindows >= 2 && root.dataset.quality !== "2") {
        root.dataset.quality = "2";
        root.classList.add("fx-low-power", "fx-no-glow", "fx-no-lens");
        root.style.setProperty("--fx-fps-cap", 30);
        root.style.setProperty("--fx-dpr-cap", 1);
        root.style.setProperty("--fx-snow-scale", 0.6);
        root.style.setProperty("--fx-blur-scale", 0.45);
        window.dispatchEvent(new CustomEvent("wishly:qualitychange", { detail: { level: 2 } }));
        window.wishlyUltraOn?.();
      }
      frames = 0;
      start = now;
    }
    requestAnimationFrame(sample);
  }

  const root = document.documentElement;
  requestAnimationFrame(sample);

  // Expose a tiny diagnostic surface for DevTools / the benchmark page.
  window.wishlyPerf = {
    quality: () => root.dataset.quality || "unknown",
    wasmBenchmark: (iterations = 500_000) => engine.benchmark(clamp(iterations, 10_000, 5_000_000)),
    hardwareConcurrency: navigator.hardwareConcurrency || "unknown",
    deviceMemory: navigator.deviceMemory || "unknown"
  };
}
boot();
