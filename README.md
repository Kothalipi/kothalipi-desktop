<p align="center">
  <img src="https://raw.githubusercontent.com/Kothalipi/kothalipi/main/assets/brand/lockups/kothalipi-horizontal-light.svg" alt="Kothalipi" width="380">
</p>

<p align="center">
  <b>Kothalipi</b>: open-source voice typing in <b>Bangla and English</b><br>
  for macOS, Windows and Linux. Hold a key, speak, and your words appear in any app.
</p>

<p align="center">
  <a href="https://kothalipi.tashanto.com">Website</a> ·
  <a href="https://kothalipi.tashanto.com/download.html">Download</a> ·
  <a href="https://kothalipi.tashanto.com/docs/index.html">Docs</a> ·
  <a href="https://github.com/Kothalipi/kothalipi">Roadmap</a>
</p>

---

## Install

```bash
curl -fsSL https://kothalipi.tashanto.com/install.sh | sh      # macOS, Linux
```

```powershell
irm https://kothalipi.tashanto.com/install.ps1 | iex           # Windows
```

## Features

- **One key per language.** Hold **Right ⌘ Command** (Windows: Right Alt) for Bangla and
  **fn** (Windows: Right Ctrl) for English. Each key picks its own language and model.
- **Offline or cloud.** Offline is free and private. Cloud uses your own Google Gemini
  (free tier) or OpenAI key, in the order you choose, and falls back to the offline model
  when every key is over its limit or you're offline.
- **Bangla models built in.** Qwen3-ASR 0.6B Bangla (recommended) and Bengali.AI Whisper
  Medium, verified by SHA-256 after download.
- **Picks models for your computer.** Setup reads your memory and chip and recommends the
  best Bangla and English model; low-memory computers are steered to Cloud.
- **Guided setup.** Permissions, listening mode, API keys with a "Get a free key" button,
  model downloads, key conflict detection on macOS, and a first dictation.
- **Light and private.** Rust backend, quiet in the tray, no account, no telemetry.

## Build

See [BUILD.md](BUILD.md). In short:

```bash
bun install
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev
```

To see every screen in a browser without building the app, run `bun run dev` and open
`http://localhost:1420/?mock`.

## Made by

**TA Shanto** · [tashanto.com](https://tashanto.com) · [GitHub](https://github.com/iamtashanto)

## Credits

Kothalipi's speech-to-text engine, hotkeys and text-insertion layer began as
[Handy](https://github.com/cjpais/Handy) by CJ Pais (MIT), which we are grateful for.
Everything else (Bangla models, language keys, cloud fallback, onboarding, interface,
branding) is Kothalipi's own. See [NOTICE.md](NOTICE.md).

## License

[MIT](LICENSE). Copyright (c) 2025 CJ Pais; (c) 2026 TA Shanto and Kothalipi contributors.
