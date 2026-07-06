# Offline Dictation with whisper.cpp

This guide configures VoxiType 0.4.0 to transcribe with a local
`whisper-cli` binary and a local GGML model file. Groq is not used for
speech-to-text in this mode.

A fully offline setup also needs the LLM engine set to `Off`, `Rule-based`,
or local `Ollama`, with translation disabled unless the selected translator is
local.

Sources checked: the official whisper.cpp README and CLI help document:
- https://github.com/ggml-org/whisper.cpp
- https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/README.md

## Quick setup path

On first launch, choose `Offline whisper.cpp` in the VoxiType setup wizard.
If the app is already configured, open `Settings -> STT` and switch
`Transcription engine` to `Offline whisper.cpp`.

VoxiType needs two local paths:

| Field | Choose this file |
| --- | --- |
| `whisper-cli path` | `whisper-cli.exe` on Windows, or `whisper-cli` on macOS/Linux |
| `Model path` | A GGML model file such as `ggml-base.bin` |

Use the `Browse` buttons in onboarding or `Settings -> STT` whenever possible.
They open the native file picker and reduce path typing mistakes.

## 1. Install whisper.cpp

### Option A: Use a prebuilt release

This is the easiest Windows setup and does not require CMake.

1. Open the official whisper.cpp releases page:
   https://github.com/ggml-org/whisper.cpp/releases
2. Download the release archive for your platform.
3. Extract it to a stable folder.
4. In VoxiType, browse to the extracted `whisper-cli.exe` file.

### Option B: Build from source

Install Git, CMake, and a C++ compiler first. On Windows, Visual Studio Build
Tools with the C++ workload is the simplest option.

```powershell
git clone https://github.com/ggml-org/whisper.cpp.git
cd whisper.cpp
cmake -B build
cmake --build build -j --config Release
```

The binary is usually one of these:

```text
Windows MSVC: build\bin\Release\whisper-cli.exe
Windows MinGW: build\bin\whisper-cli.exe
macOS/Linux: build/bin/whisper-cli
```

If `whisper-cli` is already in `PATH`, VoxiType can use `whisper-cli` as the
binary path. If not, use the full path.

## 2. Download a model

Download a GGML model from:
https://huggingface.co/ggerganov/whisper.cpp/tree/main

`base` is a good first choice for Indonesian and English dictation. `small`
is usually more accurate but slower.

If you built whisper.cpp from source, you can also download a model from the
`whisper.cpp` folder:

```bash
sh ./models/download-ggml-model.sh base
```

The model file will be similar to:

```text
models/ggml-base.bin
```

Quick model guide:

| Model | Use when |
| --- | --- |
| `tiny` | Fastest test on low-end hardware |
| `base` | Balanced first setup |
| `small` | Better accuracy if the machine is fast enough |
| `medium` or larger | Accuracy matters more than speed |

## 3. Verify outside VoxiType

Run one direct CLI test before configuring the app.

Windows example:

```powershell
.\build\bin\Release\whisper-cli.exe -m .\models\ggml-base.bin -f .\samples\jfk.wav -l auto
```

macOS/Linux example:

```bash
./build/bin/whisper-cli -m ./models/ggml-base.bin -f ./samples/jfk.wav -l auto
```

If this works, the binary and model are usable.

## 4. Configure VoxiType

Use either the first-run setup wizard or `Settings -> STT`.

1. Set `Transcription engine` to `Offline whisper.cpp`.
2. Set `whisper-cli path`.
   - Click `Browse` and choose `whisper-cli.exe` or `whisper-cli`.
   - Use `whisper-cli` only if it is already available in `PATH`.
3. Set `Model path`.
   - Click `Browse` and choose the downloaded `ggml-*.bin` file.
4. Set `Threads`.
   - Start with `4`.
   - Increase only if transcription is slow and the CPU has spare cores.
5. Set `Language`.
   - Use `Auto detect` for mixed languages.
   - Use `Bahasa Indonesia` or `English` for more predictable dictation.
6. Click `Test Offline Engine`.

The test runs a short silent WAV through the configured binary and model. If
it passes, VoxiType can execute whisper.cpp and load the model. A passing test
does not verify microphone volume, so check `Settings -> Audio` if real
dictation is empty.

## 5. Use offline dictation

After the test passes:

1. Press the configured VoxiType hotkey.
2. Speak normally.
3. Release the hotkey or stop recording.
4. VoxiType transcribes with local whisper.cpp, applies the selected LLM mode,
   applies dictionary replacements and snippets, injects the text, and saves
   history.

LLM behavior:

| Engine | Offline? | Behavior |
| --- | --- | --- |
| `Off` | Yes | Leaves STT text unchanged before dictionary and snippets |
| `Rule-based` | Yes | Applies local punctuation and cleanup rules |
| `Ollama` | Yes, if Ollama is local | Formats text through a local Ollama model |
| `Groq` | No | Sends text to Groq for formatting |

For a fully offline flow, use `Off`, `Rule-based`, or local `Ollama`, and keep
translation disabled unless the selected translation path is local.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `whisper.cpp binary not found` | Use the full path to `whisper-cli.exe`, or add its folder to `PATH`. |
| `model not found` | Use the full path to the downloaded `ggml-*.bin` file. |
| Browse does not show the file | For the binary, choose `whisper-cli.exe`. For the model, choose a `ggml-*.bin` file. |
| Test is slow the first time | This is normal; the model is loaded from disk. |
| Test passes but dictation is empty | Speak longer than one second, select the right microphone in `Settings -> Audio`, and check input volume. |
| Wrong language | Pick `Bahasa Indonesia` or `English` instead of auto-detect. |
| Fully offline still contacts cloud | Set STT to `Offline whisper.cpp`, set LLM to `Off`, `Rule-based`, or local `Ollama`, and disable cloud translation. |
