# Offline Dictation with whisper.cpp

This guide configures VoxiType to transcribe with a local `whisper-cli`
binary and a local GGML model file. Groq is not used for speech-to-text in
this mode.

Sources checked: the official whisper.cpp README and CLI help document:
- https://github.com/ggml-org/whisper.cpp
- https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/README.md

## 1. Install whisper.cpp

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
binary path.

## 2. Download a model

From the `whisper.cpp` folder, download a GGML model. `base` is a good first
choice for Indonesian and English dictation. `small` is usually more accurate
but slower.

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

Open `Settings -> STT`.

1. Set `Transcription engine` to `Offline whisper.cpp`.
2. Set `whisper-cli path`.
   - Use `whisper-cli` if it is in `PATH`.
   - Otherwise paste the full path to `whisper-cli.exe` or `whisper-cli`.
3. Set `Model path` to the full path of the `.bin` model.
4. Set `Threads`.
   - Start with `4`.
   - Increase only if transcription is slow and the CPU has spare cores.
5. Set `Language`.
   - Use `Auto detect` for mixed languages.
   - Use `Bahasa Indonesia` or `English` for more predictable dictation.
6. Click `Test Offline Engine`.

The test runs a short silent WAV through the configured binary and model. If
it passes, VoxiType can execute whisper.cpp and load the model.

## 5. Use offline dictation

After the test passes:

1. Press the configured VoxiType hotkey.
2. Speak normally.
3. Release the hotkey or stop recording.
4. VoxiType transcribes with local whisper.cpp, formats the result, applies
   dictionary replacements and snippets, injects the text, and saves history.

For a fully offline flow, set the LLM engine to `Off`, `Rule-based`, or local
`Ollama`, and keep translation disabled unless the selected LLM is local.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `whisper.cpp binary not found` | Use the full path to `whisper-cli.exe`, or add its folder to `PATH`. |
| `model not found` | Use the full path to the downloaded `ggml-*.bin` file. |
| Test is slow the first time | This is normal; the model is loaded from disk. |
| Output is empty | Speak longer than one second and check the selected microphone. |
| Wrong language | Pick `Bahasa Indonesia` or `English` instead of auto-detect. |
| Fully offline still contacts cloud | Check `Settings -> LLM` and disable Groq/translation cloud usage. |
