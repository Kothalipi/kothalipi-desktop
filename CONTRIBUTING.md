# Contributing to Kothalipi desktop

Thank you for helping! Kothalipi is open source voice typing in Bangla and English.
Issues and pull requests are welcome in **English or Bangla**.

## Ways to help

- **Test it** on your computer and report problems (see [issues](https://github.com/Kothalipi/kothalipi-desktop/issues)).
- **Wrong Bangla?** Open a _Wrong transcription_ issue with what you said and what appeared.
- **Translate** the interface into your language (`src/i18n/locales/<code>/translation.json`).
- **Code:** bugs first, then anything on the [roadmap](https://github.com/Kothalipi/kothalipi/blob/main/docs/ROADMAP.md).

## Development

Prerequisites: [Rust](https://rustup.rs/), [Bun](https://bun.sh/) and the platform tools in
[BUILD.md](BUILD.md).

```bash
bun install
mkdir -p src-tauri/resources/models
curl -L -o src-tauri/resources/models/silero-vad-v6.2-F32.gguf \
  https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/main/silero-vad-v6.2-F32.gguf
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev
```

**See the whole UI in a browser, no build needed:** `bun run dev`, then open
`http://localhost:1420/?mock` (settings) or `/?mock&onboarding` (first run). Add `&os=windows`,
`&tier=light`, `&conflicts` to try other cases.

Before opening a pull request:

```bash
bun run format && bun run lint && bun run check:translations
cd src-tauri && cargo fmt --check && cargo clippy && cargo test --lib
```

## Pull requests

- One change per PR, with a clear description of what and why.
- Add or update tests for behaviour changes.
- Every user-facing string goes through i18n (`src/i18n/locales/en/translation.json`).
- Keep Kothalipi-specific code in its own modules where possible (`kothalipi_models.rs`,
  `language_hotkeys.rs`, `cloud_stt.rs`, `key_conflicts.rs`, `device.rs`).

## Code of Conduct

Be kind and patient, especially with newcomers and non-native English speakers.
Report problems to the maintainers privately via GitHub.
