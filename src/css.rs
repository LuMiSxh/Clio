use crate::error::{ClioError, Result};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use std::collections::HashSet;

/// Combines original CSS files + extracted inline CSS, minifies via lightningcss,
/// then tree-shakes to remove rules whose selectors are entirely absent from the HTML.
pub fn build_master_css(
    original_files: &[&[u8]],
    extracted: &str,
    used: &HashSet<String>,
) -> Result<Vec<u8>> {
    // Strip @import rules: imports become invalid at non-leading positions when files are concatenated.
    let mut combined = String::new();
    for file in original_files {
        if let Ok(s) = std::str::from_utf8(file) {
            for line in s.lines() {
                if !line.trim_start().starts_with("@import") {
                    combined.push_str(line);
                    combined.push('\n');
                }
            }
        }
    }
    combined.push_str(extracted);

    let minified: String = {
        let mut sheet = StyleSheet::parse(&combined, ParserOptions::default())
            .map_err(|e| ClioError::html(format!("CSS parse: {e}")))?;
        sheet
            .minify(MinifyOptions::default())
            .map_err(|e| ClioError::html(format!("CSS minify: {e}")))?;
        sheet
            .to_css(PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            })
            .map_err(|e| ClioError::html(format!("CSS serialize: {e}")))?
            .code
    };

    Ok(swap_font_exts(&tree_shake(&minified, used)).into_bytes())
}

/// Removes top-level style rules whose selectors reference only classes/IDs absent from `used`.
/// Works on minified (whitespace-free) CSS using bracket counting.
///
/// token extraction is exact on the class/id name but ignores attribute selectors and
/// some pseudo-class arguments — keeps false positives (over-retains rules), never false negatives.
/// Tighten if retained-but-unused bloat becomes measurable.
fn tree_shake(css: &str, used: &HashSet<String>) -> String {
    let mut out = String::new();
    let mut buf = String::new();
    let mut depth: usize = 0;
    let mut in_str = false;
    let mut str_ch = '"';

    for ch in css.chars() {
        if in_str {
            buf.push(ch);
            if ch == str_ch {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_str = true;
                str_ch = ch;
                buf.push(ch);
            }
            '{' => {
                depth += 1;
                buf.push(ch);
            }
            '}' => {
                buf.push(ch);
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if rule_is_live(&buf, used) {
                        out.push_str(&buf);
                    }
                    buf.clear();
                }
            }
            _ => buf.push(ch),
        }
    }
    out.push_str(&buf); // flush anything trailing (shouldn't exist in valid CSS)
    out
}

fn swap_font_exts(css: &str) -> String {
    let mut s = css.to_owned();
    for old in &[".ttf", ".otf", ".TTF", ".OTF"] {
        for close in &[")", "\")", "')"] {
            s = s.replace(&format!("{old}{close}"), &format!(".woff2{close}"));
        }
    }
    s
}

/// Extracts all class/ID tokens from a selector string.
/// e.g. ".foo > .bar:hover" → [".foo", ".bar"]
fn extract_tokens(selector: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for individual in selector.split(',') {
        // Strip pseudo-classes/elements by taking everything before the first ':'
        let base = individual.split(':').next().unwrap_or("");
        // Split on combinator/grouping chars to get simple selector parts
        for part in base.split([' ', '>', '+', '~', '[', '(']) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            // Walk through the part collecting tokens that start with '.' or '#'
            let chars: Vec<char> = part.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '.' || chars[i] == '#' {
                    let start = i;
                    i += 1;
                    while i < chars.len()
                        && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_')
                    {
                        i += 1;
                    }
                    let token: String = chars[start..i].iter().collect();
                    if token.len() > 1 {
                        tokens.push(token);
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
    tokens
}

fn rule_is_live(rule: &str, used: &HashSet<String>) -> bool {
    let selector = rule.split('{').next().unwrap_or("").trim();
    if selector.starts_with('@') {
        return true; // always keep @media, @keyframes, @font-face, etc.
    }
    let tokens = extract_tokens(selector);
    if tokens.is_empty() {
        return true; // element/universal selector — always keep
    }
    tokens.iter().any(|t| used.contains(t.as_str()))
}
