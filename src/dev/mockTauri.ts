/**
 * Kothalipi: browser preview of the desktop UI with a mocked Tauri backend.
 *
 *   bun run dev   →   http://localhost:1420/?mock            (Settings)
 *                     http://localhost:1420/?mock&onboarding (first run)
 *                     add &os=windows or &os=linux to switch platform
 *
 * Dev only: import.meta.env.DEV guards it, so production builds drop it.
 * Fixtures come from Rust defaults (`cargo test --lib dev_fixtures -- --ignored`).
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import fixtures from "./fixtures.json";

type Json = Record<string, unknown>;

export function installMockTauri() {
  const params = new URLSearchParams(location.search);
  const os = (params.get("os") ?? "macos") as "macos" | "windows" | "linux";
  const w = window as unknown as Json;
  w.__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: os,
    os_type: os,
    family: os === "windows" ? "windows" : "unix",
    arch: "aarch64",
    version: "15.0",
    eol: "\n",
    exe_extension: os === "windows" ? "exe" : "",
  };
  mockWindows("main");

  const settings = structuredClone(fixtures.settings) as Json;
  settings.onboarding_completed = !params.has("onboarding");
  const models = structuredClone(fixtures.models) as Json[];
  const downloaded = new Set(params.has("onboarding") ? [] : [models[0].id]);
  for (const m of models) m.is_downloaded = downloaded.has(m.id as string);
  let current = params.has("onboarding") ? "" : (models[0].id as string);

  // A fake download: progress events for ~4 seconds, then done.
  const fakeDownload = (id: string, done: () => void) => {
    let pct = 0;
    const timer = setInterval(() => {
      pct = Math.min(100, pct + 7);
      const m = models.find((x) => x.id === id);
      const total = ((m?.size_mb as number) ?? 500) * 1048576;
      emit("model-download-progress", {
        model_id: id,
        downloaded: (total * pct) / 100,
        total,
        percentage: pct,
        bytes_per_sec: 12 * 1048576,
      });
      if (pct >= 100) {
        clearInterval(timer);
        if (m) m.is_downloaded = true;
        emit("model-download-complete", id);
        emit("models-updated", null);
        done();
      }
    }, 300);
  };

  const camelToSnake = (s: string) =>
    s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);

  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Json;
      switch (cmd) {
        case "get_app_settings":
        case "get_default_settings":
          return structuredClone(settings);
        case "get_available_models":
          return structuredClone(models);
        case "get_current_model":
          return current;
        case "set_active_model":
          current = a.modelId as string;
          return null;
        case "download_model":
          // Like the real command, resolves when the download finishes.
          return new Promise((resolve) =>
            fakeDownload(a.modelId as string, () => resolve(null)),
          );
        case "get_model_info":
          return models.find((m) => m.id === a.modelId) ?? null;
        case "get_history_entries":
          return { entries: [], has_more: false };
        case "get_available_microphones":
        case "get_available_output_devices":
          return [{ index: "default", name: "Default", is_default: true }];
        case "get_microphone_channels":
        case "get_active_notices":
        case "get_retired_models":
        case "get_available_accelerators":
        case "fetch_post_process_models":
          return [];
        case "check_key_conflicts":
          return {
            globe_key_busy: params.has("conflicts"),
            dictation_on_right_command: params.has("conflicts"),
            dictation_on_fn: false,
          };
        case "get_device_profile": {
          const tier = params.get("tier") ?? "high";
          const ram = { light: 4, standard: 8, high: 16 }[tier] ?? 16;
          return {
            os: os === "macos" ? "macos" : os,
            arch: "aarch64",
            chip: os === "macos" ? "Apple M2" : "Intel(R) Core(TM) i5-1135G7",
            cpu_cores: 8,
            ram_gb: ram,
            apple_silicon: os === "macos",
            tier,
          };
        }
        case "check_cloud_api_key":
          return new Promise((r) => setTimeout(() => r(null), 600));
        case "is_recording":
        case "is_portable":
        case "is_update_checks_locked":
        case "is_language_detector_downloaded":
          return false;
        case "is_laptop":
          return true;
        case "get_windows_microphone_permission_status":
          return { supported: false, overall_access: "allowed" };
        case "plugin:macos-permissions|check_accessibility_permission":
        case "plugin:macos-permissions|check_microphone_permission":
          return params.has("onboarding") ? false : true;
        case "plugin:os|locale":
          return "en-US";
        case "plugin:app|version":
          return "0.1.0";
        case "plugin:app|name":
          return "Kothalipi";
        case "plugin:store|load":
          return 1;
        case "plugin:store|get":
          return [null, false];
        case "plugin:autostart|is_enabled":
          return false;
        default:
          // change_*_setting commands update the in-memory settings.
          if (cmd.startsWith("change_") && cmd.endsWith("_setting")) {
            const key = cmd.slice("change_".length, -"_setting".length);
            const value = Object.values(a)[0];
            if (key in settings) settings[key] = value;
            return null;
          }
          if (cmd === "update_binding" || cmd === "change_binding") {
            const bindings = settings.bindings as Record<string, Json>;
            const id = (a.id ?? a.bindingId) as string;
            if (bindings[id]) bindings[id].current_binding = a.binding;
            return null;
          }
          if (cmd.startsWith("get_") && camelToSnake(cmd).endsWith("s"))
            return [];
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
