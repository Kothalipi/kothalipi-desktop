# Accuracy check

Measures how well the cloud path types what you say: word error rate (WER) and
character error rate (CER), per sentence and on average. It uses the app's own
upload code (FLAC encoding, language hints), so a change that hurts accuracy shows up here.

## 1. Make the clips

Say each sentence in `sentences.tsv` in your normal voice and save it as
`<id>.<language>.wav` (16 kHz mono) in one folder, with the sentence in
`<id>.<language>.txt`. The easiest way: dictate each one with Kothalipi
(History keeps the latest recordings under
`~/Library/Application Support/com.kothalipi.desktop/recordings`), then copy the
WAV and write the `.txt` next to it. Your own voice is the realistic test; add
your own hard sentences, names and numbers.

English clips from the Mac's voices (no microphone needed):

```bash
./scripts/eval/make-english.sh /tmp/kothalipi-eval
```

## 2. Run it

```bash
cd src-tauri
KOTHALIPI_EVAL_DIR=/tmp/kothalipi-eval KOTHALIPI_EVAL_KEY=<your Gemini key> \
  cargo test --lib live_gemini_accuracy -- --ignored --nocapture
```

Languages: `en`, `bn` (the Bangla key; English words in it are allowed) and
`bn+en` (mixed). It prints what was expected, what came back, WER, CER and time.
