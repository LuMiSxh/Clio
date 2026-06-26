# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-26

### Added

- **Image compression** — Re-encodes EPUB images to WebP (default) or AVIF using auto-tuned quality and encoder speed based on image resolution and tone. Line art and grayscale pages are treated separately from full-color illustrations. Compression pipeline shared with [Thasia](https://github.com/LuMiSxh/Thasia).
- **Duplicate image deduplication** — Identical images across chapters are hashed and encoded once, then reused across all references to avoid redundant encode work.
- **CSS tree-shaking** — All CSS files are concatenated, minified via lightningcss, and filtered: rules whose selectors don't appear in any HTML file are dropped. Output is consolidated into a single `_clio.css`.
- **Inline style extraction** — `style=""` attributes are hashed into stable generated class names and moved into the consolidated CSS file.
- **Font conversion** — TTF and OTF fonts are re-encoded to WOFF2 using a pure-Rust Brotli encoder. CSS `url()` references are updated automatically.
- **XHTML-safe HTML rewriting** — XHTML files are processed with a streaming XML parser, preserving `<?xml?>` declarations, `epub:type` attributes, and all namespace-prefixed content. Stylesheet and image references are updated in place.
- **OPF rewriting** — Image media types and font paths in the OPF manifest are updated to reflect new formats. A `<meta property="clio:processed">true</meta>` tag is injected into the metadata block.
- **Parallel processing** — Images and fonts are encoded in parallel via Rayon. HTML files are rewritten in parallel.
- **Batch directory mode** — Pass a directory to process every EPUB inside it. Output goes to `<dir>-optimized/` with original filenames preserved. Per-book stats and a grand total are shown at the end.
- **JSON output** — `--json` suppresses all streaming output and writes a single structured object to stdout on completion, suitable for programmatic use. Batch mode wraps per-book results in a `books` array with totals and error counts.
- **`--avif` flag** — Opt in to AVIF encoding instead of the default WebP. AVIF produces ~4% smaller files but is not an EPUB 3 core media type and may not render in all reading systems.
- **STORED compression for binary entries** — Already-compressed entries (WebP, AVIF, WOFF2) are written to the output ZIP as STORED rather than DEFLATED, avoiding wasted CPU on incompressible data.
- **OPF metadata namespace** — `xmlns:clio` is injected on the `<package>` element to keep the clio metadata property well-formed.
