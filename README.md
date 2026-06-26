<div align="center">

# Clio

**A command-line EPUB optimizer and compressor**

Clio compresses EPUB files by converting images to WebP or AVIF, removing unused CSS,
upgrading fonts to WOFF2, and rewriting internal HTML.
You can process single files or entire folders with a single command.

[![License](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Results](#results)

</div>

---

Named after Clio, the Greek muse of history and keeper of stories. This tool aims to preserve your EPUB library while significantly reducing its file size. It optimizes the underlying assets while keeping the formatting, structure, and readability exactly the same as the original files.

---

## Features

### Image Compression

Clio re-encodes images within the EPUB to WebP (default) or AVIF. It adjusts compression settings based on image resolution and type (treating line art differently than full-color illustrations). Duplicate images across chapters are reused to save space, often resulting in reduction rates around **~90%**.

Optionally, `--max-dim` downscales oversized images so the longest edge fits a pixel cap (using Lanczos3 resampling), which is useful when source scans are far larger than any reading device can display.

This pipeline is shared with [Thasia](https://github.com/LuMiSxh/Thasia) and has been tested across various images and use cases.

### CSS Cleanup

Merges and minifies all CSS files using [lightningcss](https://lightningcss.dev), then removes any style rules that aren't used in the book's HTML. The output is saved to a single `_clio.css` file. Inline `style=""` attributes are also extracted into generated CSS classes.

### Font Conversion

Converts TTF and OTF fonts to WOFF2 using Brotli compression, and automatically updates the respective CSS `@font-face` rules.

### HTML Rewriting

Processes XHTML files using a streaming XML parser to ensure XML declarations, `epub:type` attributes, and namespaces remain intact. Stylesheet links are updated to point to the new `_clio.css`, and image references are updated to reflect the new formats.

### Batch Processing

Point the tool at a folder to process every EPUB inside it. Optimized files are saved to a new `<directory>-optimized/` folder with their original names preserved. Summary statistics are displayed at the end.

### Metadata Tagging

Injects a `<meta property="clio:processed">true</meta>` tag into the OPF file so processed books can easily be identified.

---

## Installation

### Prebuilt binaries

Download the binary for your platform from the [latest release](https://github.com/LuMiSxh/clio/releases/latest):

| Platform              | Asset                                       |
| --------------------- | ------------------------------------------- |
| Linux (x86-64)        | `clio-<version>-x86_64-unknown-linux-gnu`   |
| macOS (Apple Silicon) | `clio-<version>-aarch64-apple-darwin`       |
| macOS (Intel)         | `clio-<version>-x86_64-apple-darwin`        |
| Windows (x86-64)      | `clio-<version>-x86_64-pc-windows-msvc.exe` |

On macOS and Linux, mark it executable and (optionally) put it on your `$PATH`:

```sh
chmod +x clio-*
# Optional:
mv clio-* /usr/local/bin/clio
```

### Build from source

#### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.85+ (edition 2024)

```sh
git clone https://github.com/LuMiSxh/clio.git
cd clio
cargo build --release
```

The binary lands at `target/release/clio`. Put it on your `$PATH` or call it directly.

---

## Usage

### Single file

```sh
# Output defaults to <stem>-optimized.epub next to the input
clio book.epub

# Explicit output path
clio book.epub /somewhere/else/book.epub

# AVIF instead of WebP — smaller files, but not in the EPUB 3 spec
clio book.epub --avif

# Cap the longest image edge to 1600px for even smaller files
clio book.epub --max-dim 1600
```

### Directory

```sh
# Processes every .epub in the folder, writes to ./my-library-optimized/
clio my-library/

clio my-library/ --avif --max-dim 1600
```

### Options

| Flag        | Description                                                                     |
| ----------- | ------------------------------------------------------------------------------- |
| `--avif`    | Encode images as AVIF instead of WebP (~4% smaller, non-standard)               |
| `--max-dim` | Cap the longest image edge to N pixels (preserves aspect ratio, off by default) |
| `--json`    | Output a single JSON object instead of streaming text                           |

> [!WARNING]
> WebP is an officially supported core media type in EPUB 3.3+. AVIF is **not** part of the EPUB 3 specification and is not guaranteed to render in all reading systems. It works in some apps like Apple Books, but use `--avif` only if you know your target reader supports it.

### What the output looks like

```
── vol-01.epub ──
Loaded: 24 images  39 html  1 css  1 opf  0 fonts
   24 images   → webp               (40.3 MB → 5.2 MB, -87%)
    1 css      → _clio.css          (3 KB → 2 KB, -43%)
   39 html    rewritten
vol-01.epub → books-optimized/vol-01.epub  [39.5 MB → 3.7 MB  (-91%)]

...

12/12 books  [452.0 MB → 40.6 MB  (-91%)]
```

### JSON output

With `--json`, nothing is streamed. A single object is written to stdout when processing completes:

```json
{
    "input": "vol-01.epub",
    "output": "books-optimized/vol-01.epub",
    "input_bytes": 39500000,
    "output_bytes": 3700000,
    "reduction_pct": 90.6,
    "images": { "count": 24, "input_bytes": 40300000, "output_bytes": 3500000 },
    "fonts": { "count": 0, "input_bytes": 0, "output_bytes": 0 },
    "css": { "count": 1, "input_bytes": 3000, "output_bytes": 1700 },
    "html": { "count": 39 }
}
```

In directory mode the top-level object wraps a `books` array and adds totals:

```json
{
  "books": [ ... ],
  "total_input_bytes": 452000000,
  "total_output_bytes": 40600000,
  "reduction_pct": 91.0,
  "ok": 12,
  "failed": 0
}
```

Failed books appear in the array as `{"input": "...", "error": "..."}` and are counted in `failed`.

---

## Results

The actual reduction rate depends on the volume of images in the source files.

| Content type                        | WebP    | AVIF    |
| ----------------------------------- | ------- | ------- |
| Image-heavy (manga, illustrated LN) | ~86–89% | ~90–93% |
| Mixed (some illustrations)          | ~45–70% | ~50–75% |
| Text-only                           | ~10–25% | ~10–30% |

CSS and HTML optimizations contribute a minor reduction regardless of book type, while text-only books benefit primarily from CSS cleanup and WOFF2 font conversion.

### Real-world example

A 15-volume illustrated light novel series (premium EPUBs, **728.8 MB** total):

| Mode                  | Output  | Reduction |
| --------------------- | ------- | --------- |
| WebP (default)        | 90.8 MB | −88%      |
| WebP `--max-dim 1600` | 61.9 MB | −92%      |
| AVIF                  | 62.3 MB | −91%      |
| AVIF `--max-dim 1600` | 44.4 MB | −94%      |

WebP with downscaling lands roughly on par with default AVIF, while staying within the EPUB 3 spec.

> [!NOTE]
> `--max-dim` is off by default and never upscales — images already within the cap are left untouched. It only ever shrinks oversized images, always preserving the original aspect ratio. Because downscaling discards pixels, it is irreversible; pick a cap that comfortably exceeds your reading device's screen resolution (e.g. `1600` for phones and most e-readers).

---

## License

BSD-3-Clause — see [LICENSE](LICENSE).

---

<div align="center">

**An open-source project by LuMiSxh**

[GitHub](https://github.com/LuMiSxh/clio) • [Issues](https://github.com/LuMiSxh/clio/issues) • [Releases](https://github.com/LuMiSxh/clio/releases)

</div>
