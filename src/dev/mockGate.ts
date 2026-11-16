/** Kothalipi: install the mocked Tauri backend for `/?mock` in dev builds only. */
import { installMockTauri } from "./mockTauri";

if (import.meta.env.DEV && new URLSearchParams(location.search).has("mock")) {
  // `&step=voices` jumps to an onboarding step (used to capture screenshots).
  const step = new URLSearchParams(location.search).get("step");
  if (step) {
    try {
      localStorage.setItem(
        "kothalipi.onboarding",
        JSON.stringify({ step, languages: ["bn", "en"] }),
      );
    } catch {
      /* storage blocked */
    }
  }
  installMockTauri();
}
