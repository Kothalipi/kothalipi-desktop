# Notice and credits

**Kothalipi** is made by **TA Shanto** ([tashanto.com](https://tashanto.com)) and contributors.

## Built on

- **[Handy](https://github.com/cjpais/Handy)** by CJ Pais and contributors, MIT licensed.
  Kothalipi's engine supervisor, global hotkey handling, text insertion and audio pipeline
  started from Handy's code. Handy's copyright notice is kept in [LICENSE](LICENSE), as the
  MIT license requires.
- **[transcribe.cpp](https://github.com/handy-computer/transcribe.cpp)** and
  **[ggml](https://github.com/ggml-org/ggml)** for on-device speech recognition.
- **[Tauri](https://tauri.app)**, React and the Rust ecosystem.

## History

This repository's Git history starts at Kothalipi 0.1.0 and lists release snapshots only. The
code those releases grew from has its own full history, with every author's commits, at
[github.com/cjpais/Handy](https://github.com/cjpais/Handy). Handy's contributors made the
engine this app is built on, and we credit them here, in [LICENSE](LICENSE), in the app's About
page and on the website.

## Models

- Bangla: [SayedShaun/Qwen3-ASR-0.6B-Bengali](https://huggingface.co/SayedShaun/Qwen3-ASR-0.6B-Bengali)
  (GGUF by sheikhmufrad) and
  [Bengali.AI Whisper medium](https://huggingface.co/bengaliAI/tugstugi_bengaliai-asr_whisper-medium)
  (ggml by winshades), both Apache-2.0.
- English and other models come from the open model catalog published for transcribe.cpp.
  Each model keeps its own license.
- Fonts: [Hind Siliguri](https://fonts.google.com/specimen/Hind+Siliguri), SIL Open Font License.
