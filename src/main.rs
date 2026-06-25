mod css;
mod encode;
mod epub;
mod error;
mod html;
mod opf;

use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "clio",
    about = "EPUB optimizer — converts images to AVIF/WebP, extracts and tree-shakes CSS"
)]
struct Cli {
    /// Input EPUB file, or directory of EPUBs (outputs to <dir>-optimized/)
    input: PathBuf,
    /// Output path (single-file mode only; default: <stem>-optimized.epub)
    output: Option<PathBuf>,
    /// Encode images as WebP instead of AVIF
    #[arg(long)]
    webp: bool,
    /// Output a single JSON object instead of streaming text
    #[arg(long)]
    json: bool,
}

fn main() {
    let cli = Cli::parse();
    let format = if cli.webp { encode::Format::Webp } else { encode::Format::Avif };

    if cli.input.is_dir() {
        batch(&cli.input, format, cli.json);
    } else {
        let output = cli.output.clone().unwrap_or_else(|| {
            let stem = cli.input.file_stem().unwrap_or_default().to_string_lossy();
            cli.input.with_file_name(format!("{stem}-optimized.epub"))
        });
        match process_file(&cli.input, &output, format, cli.json) {
            Ok((in_sz, out_sz, stats)) => {
                if cli.json {
                    println!("{}", book_json(&cli.input, &output, in_sz, out_sz, &stats));
                } else {
                    println!(
                        "{} → {}  [{} → {}{}]",
                        cli.input.display(),
                        output.display(),
                        fmt_size(in_sz),
                        fmt_size(out_sz),
                        size_change(in_sz, out_sz),
                    );
                }
            }
            Err(e) => {
                if cli.json {
                    println!("{{\"error\":{}}}", json_str(&e.to_string()));
                } else {
                    eprintln!("Error: {e}");
                }
                std::process::exit(1);
            }
        }
    }
}

fn batch(dir: &Path, format: encode::Format, json: bool) {
    let dir_name = dir.file_name().unwrap_or_default().to_string_lossy();
    let out_dir = dir.with_file_name(format!("{dir_name}-optimized"));
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("Error creating output directory: {e}");
        std::process::exit(1);
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            std::process::exit(1)
        })
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "epub").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        eprintln!("No EPUBs found in {}", dir.display());
        return;
    }

    let mut total_in = 0u64;
    let mut total_out = 0u64;
    let mut errors = 0usize;
    let mut book_jsons: Vec<String> = Vec::new();

    for entry in &entries {
        let input = entry.path();
        let output = out_dir.join(entry.file_name());
        if !json {
            println!("\n── {} ──", entry.file_name().to_string_lossy());
        }
        match process_file(&input, &output, format, json) {
            Ok((in_sz, out_sz, stats)) => {
                if json {
                    book_jsons.push(book_json(&input, &output, in_sz, out_sz, &stats));
                } else {
                    println!(
                        "{} → {}  [{} → {}{}]",
                        input.display(),
                        output.display(),
                        fmt_size(in_sz),
                        fmt_size(out_sz),
                        size_change(in_sz, out_sz),
                    );
                }
                total_in += in_sz;
                total_out += out_sz;
            }
            Err(e) => {
                if json {
                    book_jsons.push(format!(
                        r#"{{"input":{},"error":{}}}"#,
                        json_str(&input.to_string_lossy()),
                        json_str(&e.to_string()),
                    ));
                } else {
                    eprintln!("  failed: {e}");
                }
                errors += 1;
            }
        }
    }

    let ok = entries.len() - errors;
    if json {
        let pct = if total_in > 0 {
            (1.0 - total_out as f64 / total_in as f64) * 100.0
        } else {
            0.0
        };
        println!(
            r#"{{"books":[{}],"total_input_bytes":{total_in},"total_output_bytes":{total_out},"reduction_pct":{pct:.1},"ok":{ok},"failed":{errors}}}"#,
            book_jsons.join(","),
        );
    } else {
        println!(
            "\n{ok}/{} books  [{} → {}{}]",
            entries.len(),
            fmt_size(total_in),
            fmt_size(total_out),
            size_change(total_in, total_out),
        );
    }
}

fn process_file(
    input: &Path,
    output: &Path,
    format: encode::Format,
    quiet: bool,
) -> error::Result<(u64, u64, RunStats)> {
    let in_sz = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let stats = run(input, output, format, quiet)?;
    if !quiet {
        print_stat(stats.img_count, &format!("→ {}", format.ext()), stats.img_orig, stats.img_new, "images");
        print_stat(stats.font_count, "→ woff2", stats.font_orig, stats.font_new, "fonts");
        print_stat(stats.css_count, "→ _clio.css", stats.css_orig, stats.css_new, "css");
        if stats.html_count > 0 {
            println!("  {:>3} html    rewritten", stats.html_count);
        }
    }
    let out_sz = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    Ok((in_sz, out_sz, stats))
}

fn size_change(orig: u64, new: u64) -> String {
    if orig == 0 {
        return String::new();
    }
    format!("  ({:+.0}%)", (new as f64 / orig as f64 - 1.0) * 100.0)
}

fn run(
    input: &std::path::Path,
    output: &std::path::Path,
    format: encode::Format,
    quiet: bool,
) -> error::Result<RunStats> {
    use std::collections::HashSet;

    let mut assets = epub::open_epub(input)?;
    if !quiet {
        println!(
            "Loaded: {} images  {} html  {} css  {} opf  {} fonts",
            assets.images.len(),
            assets.html.len(),
            assets.css.len(),
            assets.opf.len(),
            assets.fonts.len(),
        );
    }

    let img_orig: u64 = assets.images.iter().map(|e| e.data.len() as u64).sum();
    let img_count = assets.images.len();
    assets.images = encode_images(assets.images, format)?;
    let img_new: u64 = assets.images.iter().map(|e| e.data.len() as u64).sum();

    let font_orig: u64 = assets.fonts.iter().map(|e| e.data.len() as u64).sum();
    let font_count = assets.fonts.len();
    assets.fonts = encode_fonts(assets.fonts)?;
    let font_new: u64 = assets.fonts.iter().map(|e| e.data.len() as u64).sum();

    let css_archive_path = resolve_css_path(&assets);
    let img_ext = format.ext();

    let html_results = {
        use rayon::prelude::*;
        assets
            .html
            .par_iter()
            .map(|entry| {
                let css_href = relative_path(&entry.name, &css_archive_path);
                let result = html::process_html(&entry.data, img_ext, &css_href)?;
                Ok((entry.name.clone(), result.content, result.css, result.selectors))
            })
            .collect::<error::Result<Vec<_>>>()?
    };
    let mut global_css = String::new();
    let mut global_selectors: HashSet<String> = HashSet::new();
    let html_count = html_results.len();
    assets.html = html_results
        .into_iter()
        .map(|(name, content, css, selectors)| {
            global_css.push_str(&css);
            global_selectors.extend(selectors);
            epub::EpubEntry { name, data: content }
        })
        .collect();

    let css_orig: u64 = assets.css.iter().map(|e| e.data.len() as u64).sum();
    let css_count = assets.css.len();
    let original_css: Vec<&[u8]> = assets.css.iter().map(|e| e.data.as_slice()).collect();
    let master_css = css::build_master_css(&original_css, &global_css, &global_selectors)?;
    let css_new = master_css.len() as u64;
    assets.css.clear();
    assets.css.push(epub::EpubEntry {
        name: css_archive_path.clone(),
        data: master_css,
    });

    let css_href_in_opf = assets
        .opf
        .first()
        .map(|e| relative_path(&e.name, &css_archive_path))
        .unwrap_or_else(|| css_archive_path.clone());
    for entry in &mut assets.opf {
        entry.data = opf::rewrite_opf(&entry.data, img_ext, &css_href_in_opf)?;
    }

    epub::repack_epub(&assets, output)?;

    Ok(RunStats { img_count, img_orig, img_new, font_count, font_orig, font_new, css_count, css_orig, css_new, html_count })
}

/// Encodes images in parallel, deduplicating by content hash to avoid re-encoding identical bytes.
fn encode_images(
    images: Vec<epub::EpubEntry>,
    format: encode::Format,
) -> error::Result<Vec<epub::EpubEntry>> {
    use rayon::prelude::*;
    use std::collections::HashMap;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let img_hashes: Vec<u64> = images
        .iter()
        .map(|e| {
            let mut h = DefaultHasher::new();
            e.data.hash(&mut h);
            h.finish()
        })
        .collect();

    let mut hash_to_first: HashMap<u64, usize> = HashMap::new();
    for (i, &hash) in img_hashes.iter().enumerate() {
        hash_to_first.entry(hash).or_insert(i);
    }

    let unique_indices: Vec<usize> = {
        let mut v: Vec<usize> = hash_to_first.values().copied().collect();
        v.sort_unstable();
        v
    };

    let mut encoded_map: HashMap<usize, epub::EpubEntry> = unique_indices
        .par_iter()
        .map(|&idx| {
            let e = &images[idx];
            let (name, data) = encode::process_image(&e.name, &e.data, format)?;
            Ok((idx, epub::EpubEntry { name, data }))
        })
        .collect::<error::Result<Vec<_>>>()?
        .into_iter()
        .collect();

    img_hashes
        .iter()
        .enumerate()
        .map(|(i, &hash)| {
            let first = hash_to_first[&hash];
            if first == i {
                Ok(encoded_map.remove(&first).expect("encoded entry present"))
            } else {
                // Duplicate — clone encoded bytes, swap extension on original name.
                let src = encoded_map
                    .get(&first)
                    .expect("first-occurrence entry present");
                let name = encode::swap_ext(&images[i].name, format.ext());
                Ok(epub::EpubEntry {
                    name,
                    data: src.data.clone(),
                })
            }
        })
        .collect()
}

fn encode_fonts(fonts: Vec<epub::EpubEntry>) -> error::Result<Vec<epub::EpubEntry>> {
    use rayon::prelude::*;
    fonts
        .par_iter()
        .map(|e| {
            let (name, data) = encode::font::convert_to_woff2(&e.name, &e.data)?;
            Ok(epub::EpubEntry { name, data })
        })
        .collect()
}

/// Determines the archive path for the consolidated _clio.css file.
fn resolve_css_path(assets: &epub::EpubAssets) -> String {
    assets
        .css
        .first()
        .map(|e| {
            std::path::Path::new(&e.name)
                .with_file_name("_clio.css")
                .to_string_lossy()
                .into_owned()
        })
        .or_else(|| {
            assets.html.first().and_then(|e| {
                std::path::Path::new(&e.name)
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| format!("{}/Styles/_clio.css", p.display()))
            })
        })
        .unwrap_or_else(|| "OEBPS/Styles/_clio.css".to_owned())
}

/// Relative path from `from_file`'s directory to `to_file`.
/// e.g. relative_path("OEBPS/Text/ch1.xhtml", "OEBPS/Styles/main.css") → "../Styles/main.css"
fn relative_path(from_file: &str, to_file: &str) -> String {
    let from: Vec<&str> = std::path::Path::new(from_file)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let to: Vec<&str> = to_file.split('/').filter(|s| !s.is_empty()).collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let ups = from.len() - common;
    format!("{}{}", "../".repeat(ups), to[common..].join("/"))
}

fn print_stat(count: usize, label: &str, orig: u64, new: u64, kind: &str) {
    if count == 0 {
        return;
    }
    let pct = if orig > 0 {
        (1.0 - new as f64 / orig as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "  {:>3} {kind:<8} {label:<20} ({} → {}, -{:.0}%)",
        count,
        fmt_size(orig),
        fmt_size(new),
        pct
    );
}

fn fmt_size(b: u64) -> String {
    if b >= 1_000_000 {
        format!("{:.1} MB", b as f64 / 1_000_000.0)
    } else if b >= 1_000 {
        format!("{:.0} KB", b as f64 / 1_000.0)
    } else {
        format!("{b} B")
    }
}

struct RunStats {
    img_count: usize,
    img_orig: u64,
    img_new: u64,
    font_count: usize,
    font_orig: u64,
    font_new: u64,
    css_count: usize,
    css_orig: u64,
    css_new: u64,
    html_count: usize,
}

fn book_json(input: &Path, output: &Path, in_sz: u64, out_sz: u64, s: &RunStats) -> String {
    let pct = if in_sz > 0 { (1.0 - out_sz as f64 / in_sz as f64) * 100.0 } else { 0.0 };
    format!(
        r#"{{"input":{},"output":{},"input_bytes":{in_sz},"output_bytes":{out_sz},"reduction_pct":{pct:.1},"images":{{"count":{},"input_bytes":{},"output_bytes":{}}},"fonts":{{"count":{},"input_bytes":{},"output_bytes":{}}},"css":{{"count":{},"input_bytes":{},"output_bytes":{}}},"html":{{"count":{}}}}}"#,
        json_str(&input.to_string_lossy()),
        json_str(&output.to_string_lossy()),
        s.img_count, s.img_orig, s.img_new,
        s.font_count, s.font_orig, s.font_new,
        s.css_count, s.css_orig, s.css_new,
        s.html_count,
    )
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
