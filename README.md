<div align="center">

# Clio

**A command-line EPUB optimizer and compressor**

Clio compresses EPUB files by converting images to AVIF or WebP, removing unused CSS,
upgrading fonts to WOFF2, and rewriting internal HTML.
You can process single files or entire folders with a single command.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Results](#results)

</div>

---

Named after Clio, the Greek muse of history and keeper of stories. This tool aims to preserve your EPUB library while significantly reducing its file size. It optimizes the underlying assets while keeping the formatting, structure, and readability exactly the same as the original files.

---

## Features

### Image Compression

Clio re-encodes images within the EPUB to AVIF (default) or WebP. It adjusts compression settings based on image resolution and type (treating line art differently than full-color illustrations). Duplicate images across chapters are reused to save space, often resulting in reduction rates around **~90%**.

This pipeline is shared with [Thasia](https://github.com/LuMiSxh/Thasia) and has been tested across various manga and illustrated light novels.

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

| Flag     | Description                                           |
| -------- | ----------------------------------------------------- |
| `--webp` | Encode images as WebP instead of AVIF                 |
| `--json` | Output a single JSON object instead of streaming text |

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

| Content type                        | AVIF    | WebP    |
| ----------------------------------- | ------- | ------- |
| Image-heavy (manga, illustrated LN) | ~90–93% | ~86–89% |
| Mixed (some illustrations)          | ~50–75% | ~45–70% |
| Text-only                           | ~10–30% | ~10–25% |

CSS and HTML optimizations contribute a minor reduction regardless of book type, while text-only books benefit primarily from CSS cleanup and WOFF2 font conversion.

---

## License

BSD-3-Clause — see [LICENSE](LICENSE).

---

<div align="center">

**An open-source project by LuMiSxh**

[GitHub](https://github.com/LuMiSxh/clio) • [Issues](https://github.com/LuMiSxh/clio/issues) • [Releases](https://github.com/LuMiSxh/clio/releases)

</div>
