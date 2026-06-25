use crate::error::{ClioError, Result};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use std::io::Cursor;

/// Rewrites the OPF manifest:
/// - updates image item hrefs and media-types to the converted format
/// - replaces all CSS items with a single clio-master.css entry
/// - adds xmlns:clio namespace to the root <package> element
/// - injects <meta property="clio:processed">true</meta> before </metadata>
pub fn rewrite_opf(data: &[u8], img_ext: &str, css_href: &str) -> Result<Vec<u8>> {
    let mut reader = Reader::from_reader(data);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut in_manifest = false;
    let mut css_written = false;
    let mut in_metadata = false;

    'parse: loop {
        {
            let event = reader.read_event_into(&mut buf).map_err(ClioError::html)?;
            match event {
                Event::Start(mut e) if e.name().as_ref() == b"package" => {
                    // Add xmlns:clio namespace if not already present
                    if !has_clio_namespace(&e) {
                        e.push_attribute(("xmlns:clio", "https://github.com/LuMiSxh/clio"));
                    }
                    writer
                        .write_event(Event::Start(e))
                        .map_err(ClioError::html)?;
                }
                Event::Start(e) if e.name().as_ref() == b"metadata" => {
                    in_metadata = true;
                    writer
                        .write_event(Event::Start(e))
                        .map_err(ClioError::html)?;
                }
                Event::End(e) if in_metadata && e.name().as_ref() == b"metadata" => {
                    // Inject clio:processed meta tag before </metadata>
                    write_clio_meta(&mut writer)?;
                    in_metadata = false;
                    writer.write_event(Event::End(e)).map_err(ClioError::html)?;
                }
                Event::Start(e) if e.name().as_ref() == b"manifest" => {
                    in_manifest = true;
                    writer
                        .write_event(Event::Start(e))
                        .map_err(ClioError::html)?;
                }
                Event::End(e) if in_manifest && e.name().as_ref() == b"manifest" => {
                    if !css_written {
                        write_css_item(&mut writer, css_href)?;
                    }
                    in_manifest = false;
                    writer.write_event(Event::End(e)).map_err(ClioError::html)?;
                }
                Event::Empty(e) if in_manifest && e.name().as_ref() == b"item" => {
                    process_item(e, &mut writer, img_ext, css_href, &mut css_written)?;
                }
                Event::Eof => break 'parse,
                other => writer.write_event(other).map_err(ClioError::html)?,
            }
        }
        buf.clear();
    }

    Ok(writer.into_inner().into_inner())
}

fn has_clio_namespace(elem: &BytesStart) -> bool {
    elem.attributes()
        .flatten()
        .any(|a| a.key.as_ref() == b"xmlns:clio")
}

fn write_clio_meta(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<()> {
    let mut meta = BytesStart::new("meta");
    meta.push_attribute(("property", "clio:processed"));
    writer
        .write_event(Event::Start(meta))
        .map_err(ClioError::html)?;
    writer
        .write_event(Event::Text(BytesText::new("true")))
        .map_err(ClioError::html)?;
    writer
        .write_event(Event::End(BytesEnd::new("meta")))
        .map_err(ClioError::html)?;
    Ok(())
}

fn process_item(
    elem: BytesStart<'_>,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    img_ext: &str,
    css_href: &str,
    css_written: &mut bool,
) -> Result<()> {
    let mut href = String::new();
    let mut media_type = String::new();
    let mut attrs: Vec<(String, String)> = Vec::new();

    for attr in elem.attributes() {
        let a = attr.map_err(ClioError::html)?;
        let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let v = String::from_utf8_lossy(a.value.as_ref()).into_owned();
        if k == "href" {
            href = v.clone();
        } else if k == "media-type" {
            media_type = v.clone();
        }
        attrs.push((k, v));
    }

    // CSS items: drop old ones, write clio-master.css once
    if media_type == "text/css" {
        if !*css_written {
            write_css_item(writer, css_href)?;
            *css_written = true;
        }
        return Ok(());
    }

    let src_ext = href.rsplit('.').next().unwrap_or("").to_lowercase();
    if matches!(src_ext.as_str(), "ttf" | "otf") || is_font_mt(&media_type) {
        let new_href = swap_ext(&href, "woff2");
        let mut item = BytesStart::new("item");
        for (k, v) in &attrs {
            let val: &str = match k.as_str() {
                "href" => &new_href,
                "media-type" => "font/woff2",
                _ => v,
            };
            item.push_attribute((k.as_str(), val));
        }
        writer
            .write_event(Event::Empty(item))
            .map_err(ClioError::html)?;
        return Ok(());
    }

    // Image items: swap extension and media-type
    let is_src_img = matches!(src_ext.as_str(), "jpg" | "jpeg" | "png" | "gif");
    let new_href = is_src_img.then(|| swap_ext(&href, img_ext));
    let new_mtype = is_src_img.then(|| format!("image/{img_ext}"));

    let mut item = BytesStart::new("item");
    for (k, v) in &attrs {
        let val: &str = match k.as_str() {
            "href" => new_href.as_deref().unwrap_or(v),
            "media-type" => new_mtype.as_deref().unwrap_or(v),
            _ => v,
        };
        item.push_attribute((k.as_str(), val));
    }
    writer
        .write_event(Event::Empty(item))
        .map_err(ClioError::html)?;
    Ok(())
}

fn write_css_item(writer: &mut Writer<Cursor<Vec<u8>>>, css_href: &str) -> Result<()> {
    let mut item = BytesStart::new("item");
    item.push_attribute(("id", "clio-css"));
    item.push_attribute(("href", css_href));
    item.push_attribute(("media-type", "text/css"));
    writer
        .write_event(Event::Empty(item))
        .map_err(ClioError::html)?;
    Ok(())
}

fn is_font_mt(mt: &str) -> bool {
    matches!(
        mt,
        "font/ttf"
            | "font/otf"
            | "application/font-sfnt"
            | "application/x-font-ttf"
            | "application/x-font-otf"
    )
}

fn swap_ext(name: &str, ext: &str) -> String {
    match name.rfind('.') {
        Some(i) => format!("{}.{ext}", &name[..i]),
        None => format!("{name}.{ext}"),
    }
}
