<div align="center">

# Clio

**An EPUB optimizer that actually moves the needle**

Clio converts images to AVIF or WebP, tree-shakes and consolidates CSS, upgrades fonts to WOFF2,
and rewrites HTML — all without touching the reading experience.
Single file or entire library, one command.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Results](#results)

</div>

---

In Greek mythology, Clio is the muse of history — keeper of stories, guardian of memory.
This Clio does the same thing, just with fewer megabytes.
She takes your EPUB library, compresses it down to a fraction of its size, and hands it back
looking exactly the same. No quality loss you'd notice. No format changes. No broken readers.
Just the story, lighter.

---

## Features

### Image Compression

The main event. Clio re-encodes every image in the EPUB using AVIF (default) or WebP.
It auto-tunes quality and encoder speed per image based on resolution and tone —
line art gets treated differently from full-color illustrations. Identical images across
chapters are encoded once and reused. Typical reduction: **~90%**.

This compression pipeline is the same one powering [Thasia](https://github.com/LuMiSxh/Thasia),
tested extensively against real-world manga and illustrated novel content.

### CSS Tree-Shaking

All CSS files in the EPUB are concatenated, minified via [lightningcss](https://lightningcss.dev),
and then filtered: any rule whose selectors don't appear in the actual HTML is dropped.
The result is written to a single `_clio.css` that every rewritten HTML file links to.
Inline `style=""` attributes become generated classes and move into that file too.

### Font Conversion

TTF and OTF fonts are re-encoded as WOFF2 using a pure-Rust encoder with Brotli compression.
References in CSS `url()` calls are updated automatically.

### HTML Rewriting

Every XHTML file is processed with an event-streaming XML parser so that `<?xml?>` declarations,
`epub:type` attributes, and all other namespace-prefixed content pass through untouched.
Stylesheet links are replaced with the consolidated `_clio.css` reference, and image `src`/`href`
attributes are updated to the new extension.

### Batch Mode

Point Clio at a directory and it processes every EPUB inside it, writing results to
`<directory>-optimized/` with original filenames preserved. Per-book stats, a running
failure count, and a total at the end.

### OPF Metadata

Each output EPUB gets a `<meta property="clio:processed">true</meta>` tag injected
into its OPF metadata so you can tell at a glance which files have been through the pipeline.

---

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.85+ (edition 2024)

### Build

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

# WebP instead of AVIF (better compatibility, ~4% larger)
clio book.epub --webp
```

### Directory

```sh
# Processes every .epub in the folder, writes to ./my-library-optimized/
clio my-library/

clio my-library/ --webp
```

### Options

| Flag     | Description                                             |
| -------- | ------------------------------------------------------- |
| `--webp` | Encode images as WebP instead of AVIF                   |
| `--json` | Output a single JSON object instead of streaming text   |

### What the output looks like

```
── vol-01.epub ──
Loaded: 24 images  39 html  1 css  1 opf  0 fonts
   24 images   → avif               (40.3 MB → 3.5 MB, -91%)
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
  "fonts":  { "count": 0,  "input_bytes": 0,        "output_bytes": 0 },
  "css":    { "count": 1,  "input_bytes": 3000,      "output_bytes": 1700 },
  "html":   { "count": 39 }
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

Results depend almost entirely on how image-heavy the source material is.

| Content type                        | AVIF       | WebP       |
| ----------------------------------- | ---------- | ---------- |
| Image-heavy (manga, illustrated LN) | ~90–93%    | ~86–89%    |
| Mixed (some illustrations)          | ~50–75%    | ~45–70%    |
| Text-only                           | ~10–30%    | ~10–25%    |

The CSS and HTML passes contribute a few percent on top regardless of content type.
Text-only gains come from CSS minification, tree-shaking, and WOFF2 font conversion.

---

## License

BSD-3-Clause — see [LICENSE](LICENSE).

---

<div align="center">

**An open-source project by LuMiSxh**

[GitHub](https://github.com/LuMiSxh/clio) • [Issues](https://github.com/LuMiSxh/clio/issues) • [Releases](https://github.com/LuMiSxh/clio/releases)

</div>
