# Contributing to VoxiType

Thank you for your interest in contributing! VoxiType is built by the community to provide free, private, and open-source voice-to-text.

## Development Setup

### Prerequisites
- Node.js 20+
- Rust 1.85+
- (Optional) `cmake` and LLVM for local STT/VAD compilation

### Getting Started
1. Clone the repository: `git clone https://github.com/voxitype/voxitype.git`
2. Install frontend dependencies: `npm install`
3. Run the development server: `npm run tauri dev`

By default, the dev server builds without the heavy `whisper.cpp` and `ort` dependencies. It relies on the Groq API for transcription. Ensure you add a Groq API key in the app settings to test transcription.

### Building Native Engines (Full Features)
To test local whisper.cpp and Silero VAD:
```bash
cargo build --features full
```

## Pull Request Process
1. Fork the repo and create your branch from `main`.
2. Ensure all tests pass: `cargo test` and `npx tsc --noEmit`.
3. Format your code: `cargo fmt` and `npm run format` (if available).
4. Run clippy: `cargo clippy -- -D warnings`.
5. Open a PR with a clear description of the problem and your solution.

## Architecture
Please review the `AGENTS.md` and `plan/VoxiType-TECHNICAL-PLAN.md` to understand module boundaries. VoxiType enforces strict separation between the Tauri commands layer, the state machine orchestrator, and the engine traits (STT, LLM, VAD).

## Code of Conduct
By participating in this project, you agree to abide by the [Code of Conduct](CODE_OF_CONDUCT.md).
