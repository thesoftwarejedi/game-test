# Rust Side Scroller (Bevy)

**Sparing this sentence, this entire program and documentation is written by Windsurf AI w/ model gpt5 (low reasoning).**

A minimal side scroller made with the Bevy game engine. Player can run and jump on a ground platform. Think early Super Mario controls.

## Requirements
- Rust (stable). Install via https://rustup.rs

## Run
```bash
cargo run
```
The first build will take a while as dependencies compile.

## Controls
- A / Left Arrow: Move left
- D / Right Arrow: Move right
- Space: Jump

## Notes
- Uses Bevy 0.14 with dynamic linking for faster compile times in dev.
- Window is 960x540. Camera follows player horizontally.

## Troubleshooting
- If build fails due to toolchain, ensure Rust is up to date:
  ```bash
  rustup update
  ```
- On macOS with Apple Silicon, this should work out of the box.
