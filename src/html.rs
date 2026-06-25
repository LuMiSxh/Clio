use crate::error::{ClioError, Result};
use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};
use std::collections::HashSet;
use std::io::{Cursor, Write};

pub struct ProcessedHtml {
    pub content: Vec<u8>,
    pub css: String,
    pub selectors: HashSet<String>,
}

/// Rewrites XHTML using quick-xml so that:
/// - <?xml?> declarations and all XML namespaces (epub:type etc.) are preserved
/// - <style> blocks are extracted into css
/// - style="" attrs are replaced with generated .ci-xxxx classes
/// - <link rel="stylesheet"> tags are dropped; clio-master.css is injected before </head>
/// - <img src> and <image href/xlink:href> extensions are updated
pub fn process_html(data: &[u8], img_ext: &str, css_href: &str) -> Result<ProcessedHtml> {
    let mut reader = Reader::from_reader(data);
    let mut writer: Writer<Cursor<Vec<u8>>> =
        Writer::new(Cursor::new(Vec::with_capacity(data.len())));
    let mut buf = Vec::new();
    let mut css = String::new();
    let mut selectors: HashSet<String> = HashSet::new();
    let mut in_style = false;
    let mut style_depth: u32 = 0;
    let mut skip_link = false; // for rare <link ...></link> (non-self-closing) form

    'parse: loop {
        {
            let event = reader.read_event_into(&mut buf).map_err(ClioError::html)?;
            match event {
                Event::Eof => break 'parse,

                Event::Text(t) => {
                    if in_style {
                        let text = t.unescape().map_err(ClioError::html)?;
                        css.push_str(&text);
                        css.push('\n');
                    } else if !skip_link {
                        writer
                            .write_event(Event::Text(t))
                            .map_err(ClioError::html)?;
                    }
                }

                Event::CData(c) => {
                    if in_style {
                        css.push_str(std::str::from_utf8(c.as_ref()).unwrap_or(""));
                        css.push('\n');
                    } else if !skip_link {
                        writer
                            .write_event(Event::CData(c))
                            .map_err(ClioError::html)?;
                    }
                }

                Event::Start(e) => {
                    let ename = e.name();
                    let local = local_name(ename.as_ref());
                    if in_style {
                        if local == b"style" {
                            style_depth += 1;
                        }
                    } else if skip_link {
                        // skip content inside <link></link>
                    } else if local == b"style" {
                        in_style = true;
                        style_depth = 1;
                    } else if local == b"link" && is_stylesheet(&e) {
                        skip_link = true;
                    } else {
                        emit_elem(&e, false, img_ext, &mut writer, &mut css, &mut selectors)?;
                    }
                }

                Event::End(e) => {
                    let ename = e.name();
                    let local = local_name(ename.as_ref());
                    if in_style {
                        if local == b"style" {
                            style_depth = style_depth.saturating_sub(1);
                            if style_depth == 0 {
                                in_style = false;
                            }
                        }
                    } else if skip_link {
                        if local == b"link" {
                            skip_link = false;
                        }
                    } else if local == b"head" {
                        let link = format!(
                            r#"<link rel="stylesheet" type="text/css" href="{}"/>"#,
                            css_href
                        );
                        writer
                            .get_mut()
                            .write_all(link.as_bytes())
                            .map_err(ClioError::html)?;
                        writer.write_event(Event::End(e)).map_err(ClioError::html)?;
                    } else {
                        writer.write_event(Event::End(e)).map_err(ClioError::html)?;
                    }
                }

                Event::Empty(e) => {
                    if !in_style && !skip_link {
                        let ename = e.name();
                        let local = local_name(ename.as_ref());
                        if !(local == b"link" && is_stylesheet(&e)) {
                            emit_elem(&e, true, img_ext, &mut writer, &mut css, &mut selectors)?;
                        }
                    }
                }

                // Decl (<?xml?>), PI, DocType, Comment — pass through unchanged
                other => {
                    if !skip_link {
                        writer.write_event(other).map_err(ClioError::html)?;
                    }
                }
            }
        }
        buf.clear();
    }

    Ok(ProcessedHtml {
        content: writer.into_inner().into_inner(),
        css,
        selectors,
    })
}

fn emit_elem(
    e: &BytesStart<'_>,
    is_empty: bool,
    img_ext: &str,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    css: &mut String,
    selectors: &mut HashSet<String>,
) -> Result<()> {
    let ename = e.name();
    let local = local_name(ename.as_ref());
    let is_img = local == b"img";
    let is_image = local == b"image"; // SVG <image>

    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut style_val: Option<String> = None;
    let mut has_class = false;

    for attr in e.attributes() {
        let a = attr.map_err(ClioError::html)?;
        let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        if k == "style" {
            // unescape to get real CSS text for hash/injection
            style_val = Some(a.unescape_value().map_err(ClioError::html)?.into_owned());
        } else {
            // keep raw (already-escaped) bytes so we don't double-escape on write-back
            let v = String::from_utf8_lossy(a.value.as_ref()).into_owned();
            if k == "class" {
                has_class = true;
            }
            attrs.push((k, v));
        }
    }

    let ci_class = style_val.as_ref().map(|s| {
        let cn = hash_class(s);
        css.push_str(&format!(".{cn}{{{s}}}\n"));
        cn
    });

    let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut new_e = BytesStart::new(tag.as_str());

    for (k, mut v) in attrs {
        if (is_img && k == "src") || (is_image && (k == "href" || k == "xlink:href")) {
            v = swap_img_ext(&v, img_ext);
        }
        if k == "class" {
            if let Some(ref ci) = ci_class {
                v = format!("{v} {ci}");
            }
            for c in v.split_whitespace() {
                selectors.insert(format!(".{c}"));
            }
        }
        if k == "id" {
            selectors.insert(format!("#{v}"));
        }
        new_e.push_attribute((k.as_str(), v.as_str()));
    }

    if !has_class && let Some(ci) = ci_class {
        selectors.insert(format!(".{ci}"));
        new_e.push_attribute(("class", ci.as_str()));
    }

    if is_empty {
        writer
            .write_event(Event::Empty(new_e))
            .map_err(ClioError::html)
    } else {
        writer
            .write_event(Event::Start(new_e))
            .map_err(ClioError::html)
    }
}

fn is_stylesheet(e: &BytesStart<'_>) -> bool {
    e.attributes().flatten().any(|a| {
        a.key.as_ref() == b"rel"
            && std::str::from_utf8(a.value.as_ref())
                .map(|v| v.contains("stylesheet"))
                .unwrap_or(false)
    })
}

/// Returns the local part of a possibly namespace-prefixed name.
/// b"epub:type" → b"type",  b"div" → b"div"
fn local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .rposition(|&b| b == b':')
        .map(|i| &name[i + 1..])
        .unwrap_or(name)
}

fn swap_img_ext(path: &str, new_ext: &str) -> String {
    const IMG_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "avif"];
    match path.rfind('.') {
        Some(i) => {
            let cur = path[i + 1..].to_lowercase();
            if IMG_EXTS.contains(&cur.as_str()) {
                format!("{}.{new_ext}", &path[..i])
            } else {
                path.to_owned()
            }
        }
        None => path.to_owned(),
    }
}

fn hash_class(style: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    style.trim().hash(&mut h);
    format!("c{:06x}", h.finish() & 0xFFFFFF)
}
