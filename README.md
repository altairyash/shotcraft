# Shotcraft

**A deterministic screenshot asset pipeline for developers and AI agents.**

Shotcraft turns screenshots into correctly sized, reproducible assets for articles, GitHub READMEs, and video workflows. The project is being built as a Rust-first CLI, with browser capture and desktop capture planned as separate adapters.

## Current status

The first CLI slice supports image inspection, exact-dimension validation, and composition onto preset canvases. Browser and desktop capture are not implemented yet.

## Install / run from source

Requires a recent Rust toolchain.

```bash
cargo run -- presets
cargo run -- inspect ./screenshot.png --json
cargo run -- validate ./screenshot.png --preset video-16x9 --json
cargo run -- compose ./screenshot.png --preset article --fit contain --output ./assets/article.png
```

## Presets

| Preset | Dimensions | Intended use |
|---|---:|---|
| `article` | 1200 × 630 | Article and link preview |
| `github-readme` | 1600 × 900 | Project visual for a README |
| `video-16x9` | 1920 × 1080 | Full HD video frame |
| `social` | 1200 × 630 | Social sharing card |

## Commands

- `presets` — list available output presets.
- `inspect <input> [--json]` — report dimensions, format, aspect ratio, and file size.
- `validate <input> --preset <preset> [--json]` — check that an image decodes and exactly matches the target dimensions. JSON mode is suitable for scripts and CI.
- `compose <input> --preset <preset> [--fit contain|cover] [--background #RRGGBB] --output <path>` — write a PNG on the selected canvas. `contain` is the default and avoids cropping; `cover` fills the frame and may crop edges.

## Design principles

- **Predictable output:** named presets and explicit fit behavior.
- **Agent-friendly:** machine-readable JSON reports and non-zero validation exit codes.
- **No silent cropping:** `contain` is the default; use `cover` deliberately.
- **Honest validation:** the current validator checks file decodability and dimensions, not whether the screenshot depicts the intended UI.

## Roadmap

1. Browser capture (URL, viewport, selector/element, full-page option).
2. Recipe files and structured capture reports.
3. Desktop window/screen capture with platform-specific capability checks.
4. MCP server for agent-driven capture and validation.
5. CI examples and prebuilt releases.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT (license file to be added before the first release).
