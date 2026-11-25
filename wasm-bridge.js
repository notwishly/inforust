const root = document.documentElement;
let runtimePromise;

export function startRustEngine() {
  if (runtimePromise) return runtimePromise;
  runtimePromise = (async () => {
    try {
      const wasm = await import("./pkg/wishly_engine.js");
      await wasm.default();
      const { DeviceProfile, SnowEngine, benchmark } = wasm;
      const cores = navigator.hardwareConcurrency || 4;
      const memory = navigator.deviceMemory || 4;
      const saveData = !!navigator.connection?.saveData;
      const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
      const params = new URLSearchParams(location.search);
      const forced = params.get("profile");
      const legacyLite = params.get("lite");
      let profile = new DeviceProfile(cores, memory, saveData, reducedMotion);
      if (forced === "2core" || (!forced && legacyLite === "2")) profile = new DeviceProfile(2, 2, false, reducedMotion);
      else if (forced === "4core" || (!forced && legacyLite === "1")) profile = new DeviceProfile(4, 4, false, reducedMotion);
      else if (forced === "full" || (!forced && legacyLite === "0")) profile = new DeviceProfile(8, 8, false, reducedMotion);

      if (forced === "full" || (!forced && legacyLite === "0")) window.wishlyFullOn?.();
      else if (profile.level() >= 2) window.wishlyUltraOn?.();
      else if (profile.level() === 1) window.wishlyLiteOn?.();

      root.dataset.quality = String(profile.level());
      root.style.setProperty("--fx-fps-cap", profile.fps_cap());
      root.style.setProperty("--fx-dpr-cap", profile.dpr_cap());
      root.style.setProperty("--fx-snow-scale", profile.snow_scale());
      root.style.setProperty("--fx-blur-scale", profile.blur_scale());
      root.classList.toggle("fx-no-glow", !profile.glow_enabled());
      root.classList.toggle("fx-no-lens", !profile.lens_enabled());

      return { profile, snow: new SnowEngine(), benchmark: (n = 500_000) => benchmark(n) };
    } catch (error) {
      console.warn("[Wishly] WASM unavailable; Rust snow layer was not started.", error);
      runtimePromise = null;
      return null;
    }
  })();
  return runtimePromise;
}
