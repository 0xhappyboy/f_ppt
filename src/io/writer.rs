//! Presentation serialization logic.
use crate::core::media::FMediaType;
use crate::core::presentation::FPresentation;
use crate::core::shape::*;
use crate::core::slide::{FBackgroundType, FSlide};
use crate::core::table::FTable;
use crate::core::text::*;
use crate::{FTableRow, encode_xml_text};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use zip::ZipWriter;
use zip::write::FileOptions;

/// XML declaration shared by every serialized part.
const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// Package root relationships part.
const ROOT_RELS_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/>",
    "</Relationships>"
);

/// Per-slide image reference resolved for serialization.
#[derive(Clone)]
struct SlideImageRef {
    new_rid: String,
    filename: String,
}

/// Image serialization plan shared across the package.
struct ImagePlan {
    /// Mapping from original relationship id to new relationship id per slide.
    per_slide: Vec<HashMap<String, SlideImageRef>>,
    /// Media files keyed by target file name.
    files: Vec<(String, Vec<u8>)>,
}

/// Serializes a complete presentation to bytes.
pub(crate) fn serialize_presentation(pres: &FPresentation) -> crate::FPptResult<Vec<u8>> {
    println!(
        "[DEBUG] serialize_presentation: starting with {} slides",
        pres.slide_count()
    );

    let plan = build_image_plan(pres);

    let cursor = std::io::Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options: FileOptions<'_, '_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let has_theme = pres.theme.raw_xml.is_some() || pres.theme.name.is_some();
    println!("[DEBUG] has_theme: {}", has_theme);

    // Required package parts.
    writer.start_file("[Content_Types].xml", options)?;
    let media_extensions: Vec<String> = plan
        .files
        .iter()
        .map(|(name, _)| file_extension(name))
        .collect();
    let ct_xml = serialize_content_types(pres.slide_count(), has_theme, &media_extensions);
    println!("[DEBUG] [Content_Types].xml length: {}", ct_xml.len());
    writer.write_all(ct_xml.as_bytes())?;
    println!("[DEBUG] [Content_Types].xml written");

    writer.start_file("_rels/.rels", options)?;
    writer.write_all(ROOT_RELS_XML.as_bytes())?;
    println!("[DEBUG] _rels/.rels written");

    // Presentation relationships returns tuple: (xml_string, theme_rid, master_rid, layout_rid)
    println!("[DEBUG] calling serialize_presentation_rels()...");
    let (pres_rels_xml, theme_rid, master_rid, _layout_rid) = serialize_presentation_rels(pres);
    println!("[DEBUG] serialize_presentation_rels() returned");
    println!("[DEBUG] theme_rid: {:?}, master_rid: {:?}", theme_rid, master_rid);
    
    println!("[DEBUG] calling serialize_presentation_xml()...");
    let pres_xml = serialize_presentation_xml(pres, master_rid.as_deref());
    println!("[DEBUG] serialize_presentation_xml() returned, length: {}", pres_xml.len());
    
    writer.start_file("ppt/presentation.xml", options)?;
    writer.write_all(pres_xml.as_bytes())?;
    println!("[DEBUG] presentation.xml written");

    println!(
        "[DEBUG] presentation.xml.rels length: {}",
        pres_rels_xml.len()
    );
    writer.start_file("ppt/_rels/presentation.xml.rels", options)?;
    writer.write_all(pres_rels_xml.as_bytes())?;
    println!("[DEBUG] presentation.xml.rels written");

    // Theme.
    if has_theme {
        let theme_xml = serialize_theme(&pres.theme);
        println!("[DEBUG] theme1.xml length: {}", theme_xml.len());
        writer.start_file("ppt/theme/theme1.xml", options)?;
        writer.write_all(theme_xml.as_bytes())?;
        println!("[DEBUG] theme1.xml written");
    }

    // Slide master (required by OOXML spec).
    println!("[DEBUG] calling serialize_slide_master()...");
    let slide_master_xml = serialize_slide_master();
    println!("[DEBUG] serialize_slide_master() returned, length: {}", slide_master_xml.len());
    writer.start_file("ppt/slideMasters/slideMaster1.xml", options)?;
    println!("[DEBUG] slideMaster1.xml start_file ok");
    writer.write_all(slide_master_xml.as_bytes())?;
    println!("[DEBUG] slideMaster1.xml written");

    println!("[DEBUG] calling serialize_slide_master_rels()...");
    let slide_master_rels_xml = serialize_slide_master_rels(has_theme);
    println!("[DEBUG] serialize_slide_master_rels() returned, length: {}", slide_master_rels_xml.len());
    writer.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)?;
    println!("[DEBUG] slideMaster1.xml.rels start_file ok");
    writer.write_all(slide_master_rels_xml.as_bytes())?;
    println!("[DEBUG] slideMaster1.xml.rels written");

    // Slide layout (required by OOXML spec).
    println!("[DEBUG] calling serialize_slide_layout()...");
    let slide_layout_xml = serialize_slide_layout();
    println!("[DEBUG] serialize_slide_layout() returned, length: {}", slide_layout_xml.len());
    writer.start_file("ppt/slideLayouts/slideLayout1.xml", options)?;
    println!("[DEBUG] slideLayout1.xml start_file ok");
    writer.write_all(slide_layout_xml.as_bytes())?;
    println!("[DEBUG] slideLayout1.xml written");

    println!("[DEBUG] calling serialize_slide_layout_rels()...");
    let slide_layout_rels_xml = serialize_slide_layout_rels();
    println!("[DEBUG] serialize_slide_layout_rels() returned, length: {}", slide_layout_rels_xml.len());
    writer.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)?;
    println!("[DEBUG] slideLayout1.xml.rels start_file ok");
    writer.write_all(slide_layout_rels_xml.as_bytes())?;
    println!("[DEBUG] slideLayout1.xml.rels written");

    // Slides.
    for (idx, slide_id) in pres.slides.iter().enumerate() {
        let slide_num = idx + 1;
        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);

        let empty = HashMap::new();
        let slide_images = plan.per_slide.get(idx).unwrap_or(&empty);

        let slide_xml = match &slide_id.slide {
            Some(slide) => serialize_slide(slide, slide_images),
            None => serialize_empty_slide(),
        };
        println!("[DEBUG] slide{}.xml length: {}", slide_num, slide_xml.len());
        writer.start_file(slide_path.as_str(), options)?;
        writer.write_all(slide_xml.as_bytes())?;
        println!("[DEBUG] slide{}.xml written", slide_num);

        // Only generate .rels file if there are image relationships
        if !slide_images.is_empty() {
            let slide_rels_xml = serialize_slide_rels(slide_images);
            println!(
                "[DEBUG] slide{}.xml.rels length: {} ({} images)",
                slide_num,
                slide_rels_xml.len(),
                slide_images.len()
            );
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
            writer.start_file(rels_path.as_str(), options)?;
            writer.write_all(slide_rels_xml.as_bytes())?;
            println!("[DEBUG] slide{}.xml.rels written", slide_num);
        }
    }

    // Media parts.
    for (name, data) in &plan.files {
        let path = format!("ppt/media/{}", name);
        println!(
            "[DEBUG] writing media part: {} ({} bytes)",
            path,
            data.len()
        );
        writer.start_file(path.as_str(), options)?;
        writer.write_all(data)?;
    }
    println!("[DEBUG] all media parts written");

    // Preserved parts (everything else).
    for part in &pres.package.parts {
        if part.path == "[Content_Types].xml"
            || part.path == "_rels/.rels"
            || part.path == "ppt/presentation.xml"
            || part.path == "ppt/_rels/presentation.xml.rels"
            || part.path.starts_with("ppt/slides/")
            || part.path.starts_with("ppt/media/")
            || part.path.starts_with("ppt/slideMasters/")
            || part.path.starts_with("ppt/slideLayouts/")
            || (has_theme && part.path == "ppt/theme/theme1.xml")
        {
            continue;
        }
        println!(
            "[DEBUG] writing preserved part: {} ({} bytes)",
            part.path,
            part.data.len()
        );
        writer.start_file(part.path.as_str(), options)?;
        writer.write_all(&part.data)?;
    }
    println!("[DEBUG] all preserved parts written");

    println!("[DEBUG] calling writer.finish()...");
    let cursor = writer.finish()?;
    println!(
        "[DEBUG] serialize_presentation: finished, total bytes: {}",
        cursor.get_ref().len()
    );
    Ok(cursor.into_inner())
}

/// Builds the image plan by scanning all slides for pictures and matching them to media parts.
fn build_image_plan(pres: &FPresentation) -> ImagePlan {
    // Map from normalized package path (e.g. ppt/media/image1.jpg) to bytes.
    let mut media_parts: HashMap<String, Vec<u8>> = HashMap::new();
    for part in &pres.package.parts {
        if part.path.starts_with("ppt/media/") {
            media_parts.insert(part.path.clone(), part.data.clone());
        }
    }

    println!(
        "[DEBUG] build_image_plan: found {} media parts",
        media_parts.len()
    );
    for (path, _) in &media_parts {
        println!("[DEBUG]   media part: {}", path);
    }

    let mut target_to_name: HashMap<String, String> = HashMap::new();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut used_names: HashSet<String> = HashSet::new();
    let mut counter: u32 = 0;

    let mut per_slide: Vec<HashMap<String, SlideImageRef>> = Vec::new();

    for (idx, slide_id) in pres.slides.iter().enumerate() {
        let mut map: HashMap<String, SlideImageRef> = HashMap::new();
        if let Some(slide) = &slide_id.slide {
            collect_slide_images(
                &slide.shapes,
                &media_parts,
                &mut target_to_name,
                &mut files,
                &mut used_names,
                &mut counter,
                &mut map,
            );
        }
        println!("[DEBUG] slide {} has {} images", idx + 1, map.len());
        per_slide.push(map);
    }

    // Preserve unreferenced media parts with original basenames.
    for part in &pres.package.parts {
        if !part.path.starts_with("ppt/media/") {
            continue;
        }
        let basename = part.path.trim_start_matches("ppt/media/").to_string();
        if used_names.contains(&basename) {
            continue;
        }
        used_names.insert(basename.clone());
        files.push((basename, part.data.clone()));
    }

    ImagePlan { per_slide, files }
}

/// Recursively collects images from shapes into the plan.
fn collect_slide_images(
    shapes: &[FShape],
    media_parts: &HashMap<String, Vec<u8>>,
    target_to_name: &mut HashMap<String, String>,
    files: &mut Vec<(String, Vec<u8>)>,
    used_names: &mut HashSet<String>,
    counter: &mut u32,
    map: &mut HashMap<String, SlideImageRef>,
) {
    for shape in shapes {
        match shape {
            FShape::Picture(pic) => {
                if map.contains_key(&pic.relationship_id) {
                    continue;
                }

                let resolved = resolve_picture_media(pic, media_parts);
                let (key, bytes) = match resolved {
                    Some(v) => v,
                    None => {
                        println!(
                            "[DEBUG] failed to resolve media for picture: rid={}, target={:?}",
                            pic.relationship_id, pic.target
                        );
                        continue;
                    }
                };

                let filename = if let Some(name) = target_to_name.get(&key) {
                    name.clone()
                } else {
                    let ext = file_extension(&key);
                    let mut name = String::new();
                    loop {
                        *counter += 1;
                        name = format!("image{}.{}", counter, ext);
                        if !used_names.contains(&name) {
                            break;
                        }
                    }
                    used_names.insert(name.clone());
                    target_to_name.insert(key.clone(), name.clone());
                    files.push((name.clone(), bytes));
                    name
                };

                let new_rid = format!("rIdImg{}", map.len() + 1);
                println!(
                    "[DEBUG] mapped picture {} -> {} ({})",
                    pic.relationship_id, new_rid, filename
                );
                map.insert(
                    pic.relationship_id.clone(),
                    SlideImageRef { new_rid, filename },
                );
            }
            FShape::Group(g) => {
                collect_slide_images(
                    &g.shapes,
                    media_parts,
                    target_to_name,
                    files,
                    used_names,
                    counter,
                    map,
                );
            }
            _ => {}
        }
    }
}

/// Resolves the media part and key for a picture.
fn resolve_picture_media(
    pic: &FPicture,
    media_parts: &HashMap<String, Vec<u8>>,
) -> Option<(String, Vec<u8>)> {
    // Preferred path: explicit target.
    if let Some(target) = &pic.target {
        let normalized = normalize_target(target);
        println!(
            "[DEBUG] trying to resolve target: {} -> {}",
            target, normalized
        );
        if let Some(bytes) = media_parts.get(&normalized) {
            return Some((normalized, bytes.clone()));
        }
    }

    // Fallback: derive expected filename from relationship id (rIdImg1 -> image1.*).
    let digits: String = pic
        .relationship_id
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if !digits.is_empty() {
        let prefix = format!("ppt/media/image{}.", digits);
        for (path, data) in media_parts {
            if path.starts_with(&prefix) {
                return Some((path.clone(), data.clone()));
            }
        }
    }

    // Fallback: single media part if only one is present.
    if media_parts.len() == 1 {
        if let Some((path, data)) = media_parts.iter().next() {
            return Some((path.clone(), data.clone()));
        }
    }

    None
}

/// Normalizes a relationship target to a package path.
fn normalize_target(target: &str) -> String {
    let mut t = target.trim();
    // Remove leading ../
    while let Some(stripped) = t.strip_prefix("../") {
        t = stripped;
    }
    // Remove leading /
    t = t.trim_start_matches('/');
    // Add ppt/ prefix if not present
    if t.starts_with("ppt/") {
        t.to_string()
    } else {
        format!("ppt/{}", t)
    }
}

/// Extracts a lowercase file extension from a path.
fn file_extension(path: &str) -> String {
    path.rsplit('.').next().unwrap_or("png").to_lowercase()
}

/// Serializes the package content types part.
fn serialize_content_types(
    slide_count: usize,
    has_theme: bool,
    media_extensions: &[String],
) -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">");
    out.push_str("<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>");
    out.push_str("<Default Extension=\"xml\" ContentType=\"application/xml\"/>");

    let mut declared: HashSet<String> = HashSet::new();
    declared.insert("rels".to_string());
    declared.insert("xml".to_string());

    let base: &[(&str, &str)] = &[
        ("jpeg", "image/jpeg"),
        ("jpg", "image/jpeg"),
        ("png", "image/png"),
        ("gif", "image/gif"),
        ("bmp", "image/bmp"),
        ("tiff", "image/tiff"),
        ("tif", "image/tiff"),
        ("svg", "image/svg+xml"),
        ("mp4", "video/mp4"),
    ];
    for (ext, ct) in base {
        out.push_str(&format!(
            "<Default Extension=\"{}\" ContentType=\"{}\"/>",
            ext, ct
        ));
        declared.insert(ext.to_string());
    }

    for ext in media_extensions {
        if declared.insert(ext.clone()) {
            let ct = match ext.as_str() {
                "jpeg" | "jpg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                "tiff" | "tif" => "image/tiff",
                "svg" => "image/svg+xml",
                "mp4" => "video/mp4",
                _ => "application/octet-stream",
            };
            out.push_str(&format!(
                "<Default Extension=\"{}\" ContentType=\"{}\"/>",
                ext, ct
            ));
        }
    }

    out.push_str("<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>");
    // Slide master and layout are required parts
    out.push_str("<Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>");
    out.push_str("<Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>");
    if has_theme {
        out.push_str("<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>");
    }
    for i in 1..=slide_count {
        out.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>",
            i
        ));
    }
    out.push_str("</Types>");
    out
}

/// Serializes the main presentation part.
fn serialize_presentation_xml(pres: &FPresentation, master_rid: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">");
    // sldMasterIdLst must come before sldIdLst
    if let Some(rid) = master_rid {
        out.push_str(&format!(
            "<p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"{}\"/></p:sldMasterIdLst>",
            encode_xml_text(rid)
        ));
    }
    out.push_str("<p:sldIdLst>");
    for (idx, slide_id) in pres.slides.iter().enumerate() {
        let rid = if slide_id.relationship_id.is_empty() {
            format!("rId{}", idx + 1)
        } else {
            slide_id.relationship_id.clone()
        };
        out.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"{}\"/>",
            256 + idx as u32,
            encode_xml_text(&rid)
        ));
    }
    out.push_str("</p:sldIdLst>");
    out.push_str(&format!(
        "<p:sldSz cx=\"{}\" cy=\"{}\"{} />",
        pres.slide_size.width,
        pres.slide_size.height,
        pres.slide_size
            .size_type
            .as_ref()
            .map(|t| format!(" type=\"{}\"", encode_xml_text(t)))
            .unwrap_or_default()
    ));
    out.push_str("<p:notesSz cx=\"6858000\" cy=\"9144000\"/>");
    out.push_str("</p:presentation>");
    out
}

/// Serializes presentation relationships.
/// Returns (rels_xml, theme_rid, master_rid, layout_rid)
fn serialize_presentation_rels(pres: &FPresentation) -> (String, Option<String>, Option<String>, Option<String>) {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );

    let mut used_ids: HashSet<String> = HashSet::new();
    let mut next_id = 1u32;

    // Slide master relationship (required)
    let master_rid = format!("rId{}", next_id);
    used_ids.insert(master_rid.clone());
    next_id += 1;
    out.push_str(&format!(
        "<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>",
        encode_xml_text(&master_rid)
    ));

    // Note: slideLayout is NOT linked from presentation.xml.rels
    // It is only linked from slideMaster1.xml.rels

    for (idx, slide_id) in pres.slides.iter().enumerate() {
        // Generate a unique rid, don't use the original one if it conflicts
        let mut rid = if slide_id.relationship_id.is_empty() {
            format!("rId{}", next_id)
        } else {
            slide_id.relationship_id.clone()
        };
        // Ensure uniqueness - if conflict, generate new one
        while used_ids.contains(&rid) {
            rid = format!("rId{}", next_id);
            next_id += 1;
        }
        used_ids.insert(rid.clone());
        next_id += 1;
        out.push_str(&format!(
            "<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>",
            encode_xml_text(&rid),
            idx + 1
        ));
    }

    let mut theme_rid: Option<String> = None;
    if pres.theme.raw_xml.is_some() || pres.theme.name.is_some() {
        let mut trid = format!("rId{}", next_id);
        while used_ids.contains(&trid) {
            next_id += 1;
            trid = format!("rId{}", next_id);
        }
        used_ids.insert(trid.clone());
        out.push_str(&format!(
            "<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"theme/theme1.xml\"/>",
            encode_xml_text(&trid)
        ));
        theme_rid = Some(trid);
    }

    out.push_str("</Relationships>");
    (out, theme_rid, Some(master_rid), None)
}

/// Serializes slide relationships for images.
fn serialize_slide_rels(slide_images: &HashMap<String, SlideImageRef>) -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );

    let mut entries: Vec<&SlideImageRef> = slide_images.values().collect();
    entries.sort_by(|a, b| a.new_rid.cmp(&b.new_rid));

    for img in entries {
        out.push_str(&format!(
            "<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"../media/{}\"/>",
            encode_xml_text(&img.new_rid),
            encode_xml_text(&img.filename)
        ));
    }

    out.push_str("</Relationships>");
    out
}

/// Serializes an empty slide.
fn serialize_empty_slide() -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">");
    out.push_str("<p:cSld><p:spTree>");
    out.push_str(
        "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>",
    );
    out.push_str("<p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>");
    out.push_str("</p:spTree></p:cSld>");
    out.push_str("<p:clrMapOvr><a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/></p:clrMapOvr>");
    out.push_str("</p:sld>");
    out
}

/// Serializes a slide.
fn serialize_slide(slide: &FSlide, slide_images: &HashMap<String, SlideImageRef>) -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">");
    out.push_str("<p:cSld>");
    if let Some(bg) = &slide.background {
        out.push_str(&serialize_background(bg));
    }
    out.push_str("<p:spTree>");
    out.push_str(
        "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>",
    );
    out.push_str("<p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>");
    for shape in &slide.shapes {
        if let Some(xml) = serialize_shape(shape, slide_images) {
            out.push_str(&xml);
        }
    }
    out.push_str("</p:spTree>");
    out.push_str("</p:cSld>");
    // clrMapOvr must come before transition
    out.push_str("<p:clrMapOvr><a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/></p:clrMapOvr>");
    if let Some(trans) = &slide.transition {
        out.push_str(&serialize_transition(trans));
    }
    out.push_str("</p:sld>");
    out
}

/// Serializes slide background.
fn serialize_background(bg: &crate::core::slide::FBackground) -> String {
    let mut out = String::from("<p:bg><p:bgPr>");
    match bg.fill_type {
        Some(FBackgroundType::Solid) => {
            let color = bg.color.clone().unwrap_or_else(|| "FFFFFF".to_string());
            out.push_str(&format!(
                "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
                encode_xml_text(&color)
            ));
        }
        Some(FBackgroundType::Gradient) => {
            out.push_str("<a:gradFill><a:gsLst><a:gs pos=\"0\"><a:srgbClr val=\"FFFFFF\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"000000\"/></a:gs></a:gsLst><a:lin ang=\"5400000\" scaled=\"1\"/></a:gradFill>");
        }
        Some(FBackgroundType::Picture) => {
            out.push_str("<a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill>");
        }
        _ => {
            out.push_str("<a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill>");
        }
    }
    out.push_str("<a:effectLst/></p:bgPr></p:bg>");
    out
}

/// Serializes slide transition.
fn serialize_transition(trans: &crate::core::slide::FSlideTransition) -> String {
    let mut out = String::from("<p:transition");
    if let Some(adv) = trans.advance_on_click {
        out.push_str(&format!(" advClick=\"{}\"", if adv { "1" } else { "0" }));
    }
    if let Some(tm) = trans.advance_after_ms {
        out.push_str(&format!(" advTm=\"{}\"", tm));
    }
    out.push('>');
    let trans_type = trans
        .transition_type
        .clone()
        .unwrap_or_else(|| "fade".to_string());
    match trans_type.as_str() {
        "push" => out.push_str("<p:push/></p:transition>"),
        "wipe" => out.push_str("<p:wipe/></p:transition>"),
        _ => out.push_str("<p:fade/></p:transition>"),
    }
    out
}

/// Serializes a shape.
fn serialize_shape(
    shape: &FShape,
    slide_images: &HashMap<String, SlideImageRef>,
) -> Option<String> {
    match shape {
        FShape::TextBox(tb) => Some(serialize_text_box(tb)),
        FShape::AutoShape(auto) => Some(serialize_auto_shape(auto)),
        FShape::Picture(pic) => serialize_picture(pic, slide_images),
        FShape::Group(grp) => Some(serialize_group(grp, slide_images)),
        FShape::Connector(cxn) => Some(serialize_connector(cxn)),
        FShape::GraphicFrame(gf) => serialize_graphic_frame(gf),
        FShape::Placeholder(ph) => Some(serialize_placeholder(ph)),
    }
}

/// Serializes an xfrm element with rotation and flip attributes (a:xfrm namespace).
fn serialize_xfrm(props: &FShapeProperties) -> String {
    let mut out = String::from("<a:xfrm");
    if let Some(rot) = props.rotation {
        out.push_str(&format!(" rot=\"{}\"", rot));
    }
    if props.flip_horizontal {
        out.push_str(" flipH=\"1\"");
    }
    if props.flip_vertical {
        out.push_str(" flipV=\"1\"");
    }
    out.push_str(&format!(
        "><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>",
        props.x, props.y, props.width, props.height
    ));
    out
}

/// Serializes a p:xfrm element for graphicFrame (must use p: namespace).
fn serialize_p_xfrm(props: &FShapeProperties) -> String {
    let mut out = String::from("<p:xfrm");
    if let Some(rot) = props.rotation {
        out.push_str(&format!(" rot=\"{}\"", rot));
    }
    if props.flip_horizontal {
        out.push_str(" flipH=\"1\"");
    }
    if props.flip_vertical {
        out.push_str(" flipV=\"1\"");
    }
    out.push_str(&format!(
        "><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm>",
        props.x, props.y, props.width, props.height
    ));
    out
}

/// Serializes shape properties with a preset rectangle geometry.
fn serialize_shape_properties(props: &FShapeProperties) -> String {
    let mut out = String::from("<p:spPr>");
    out.push_str(&serialize_xfrm(props));
    out.push_str("<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>");
    out.push_str("</p:spPr>");
    out
}

/// Serializes shape properties without geometry (for pictures and frames).
fn serialize_shape_properties_no_geom(props: &FShapeProperties) -> String {
    let mut out = String::from("<p:spPr>");
    out.push_str(&serialize_xfrm(props));
    out.push_str("</p:spPr>");
    out
}

/// Serializes a text box.
fn serialize_text_box(tb: &FShapeContainer) -> String {
    let mut out = String::new();
    out.push_str("<p:sp>");
    out.push_str(&format!(
        "<p:nvSpPr><p:cNvPr id=\"{}\" name=\"{}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>",
        tb.id,
        encode_xml_text(&tb.name)
    ));
    out.push_str(&serialize_shape_properties(&tb.properties));
    out.push_str(&serialize_text_body(&tb.text_body));
    out.push_str("</p:sp>");
    out
}

/// Serializes an auto shape.
fn serialize_auto_shape(auto: &FAutoShape) -> String {
    let mut out = String::new();
    out.push_str("<p:sp>");
    out.push_str(&format!(
        "<p:nvSpPr><p:cNvPr id=\"{}\" name=\"{}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>",
        auto.id,
        encode_xml_text(&auto.name)
    ));
    out.push_str("<p:spPr>");
    out.push_str(&serialize_xfrm(&auto.properties));
    out.push_str(&format!(
        "<a:prstGeom prst=\"{}\"><a:avLst/></a:prstGeom>",
        auto_shape_type_to_str(auto.shape_type)
    ));
    if let Some(fill) = &auto.fill_color {
        out.push_str(&format!(
            "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            encode_xml_text(fill)
        ));
    }
    if let Some(line) = &auto.line_color {
        let width = auto.line_width.unwrap_or(9525);
        out.push_str(&format!(
            "<a:ln w=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></a:ln>",
            width,
            encode_xml_text(line)
        ));
    }
    out.push_str("</p:spPr>");
    out.push_str(&serialize_text_body(&auto.text_body));
    out.push_str("</p:sp>");
    out
}

/// Converts auto shape type to string.
fn auto_shape_type_to_str(t: FAutoShapeType) -> &'static str {
    match t {
        FAutoShapeType::Rectangle => "rect",
        FAutoShapeType::Ellipse => "ellipse",
        FAutoShapeType::Triangle => "triangle",
        FAutoShapeType::RightTriangle => "rtTriangle",
        FAutoShapeType::Diamond => "diamond",
        FAutoShapeType::Pentagon => "pentagon",
        FAutoShapeType::Hexagon => "hexagon",
        FAutoShapeType::Octagon => "octagon",
        FAutoShapeType::Star5 => "star5",
        FAutoShapeType::Star6 => "star6",
        FAutoShapeType::Arrow => "arrow",
        FAutoShapeType::Line => "line",
        FAutoShapeType::RoundRectangle => "roundRect",
        FAutoShapeType::SnipRectangle => "snipRect",
        FAutoShapeType::Parallelogram => "parallelogram",
        FAutoShapeType::Trapezoid => "trapezoid",
        FAutoShapeType::Heart => "heart",
        FAutoShapeType::LightningBolt => "lightningBolt",
        FAutoShapeType::Sun => "sun",
        FAutoShapeType::Moon => "moon",
        FAutoShapeType::Cloud => "cloud",
        FAutoShapeType::Arc => "arc",
        FAutoShapeType::Pie => "pie",
        FAutoShapeType::Donut => "donut",
        FAutoShapeType::NoSymbol => "noSmoking",
        FAutoShapeType::BlockArc => "blockArc",
        FAutoShapeType::FoldedCorner => "foldedCorner",
        FAutoShapeType::SmileyFace => "smileyFace",
        FAutoShapeType::Teardrop => "teardrop",
        FAutoShapeType::Trapezoid2 => "trapezoid2",
        FAutoShapeType::IrregularSeal1 => "irregularSeal1",
        FAutoShapeType::IrregularSeal2 => "irregularSeal2",
        FAutoShapeType::LeftRightArrow => "leftRightArrow",
        FAutoShapeType::UpDownArrow => "upDownArrow",
        FAutoShapeType::LeftArrow => "leftArrow",
        FAutoShapeType::RightArrow => "rightArrow",
        FAutoShapeType::UpArrow => "upArrow",
        FAutoShapeType::DownArrow => "downArrow",
        FAutoShapeType::BendedArrow => "bentArrow",
        FAutoShapeType::UTurnArrow => "uturnArrow",
        FAutoShapeType::LeftUpArrow => "leftUpArrow",
        FAutoShapeType::BentUpArrow => "bentUpArrow",
        FAutoShapeType::CurvedRightArrow => "curvedRightArrow",
        FAutoShapeType::CurvedLeftArrow => "curvedLeftArrow",
        FAutoShapeType::CurvedUpArrow => "curvedUpArrow",
        FAutoShapeType::CurvedDownArrow => "curvedDownArrow",
        FAutoShapeType::StripedRightArrow => "stripedRightArrow",
        FAutoShapeType::NotchedRightArrow => "notchedRightArrow",
        FAutoShapeType::PentagonArrow => "pentagonArrow",
        FAutoShapeType::Chevron => "chevron",
        FAutoShapeType::RightArrowCallout => "rightArrowCallout",
        FAutoShapeType::DownArrowCallout => "downArrowCallout",
        FAutoShapeType::LeftArrowCallout => "leftArrowCallout",
        FAutoShapeType::UpArrowCallout => "upArrowCallout",
        FAutoShapeType::LeftRightArrowCallout => "leftRightArrowCallout",
        FAutoShapeType::UpDownArrowCallout => "upDownArrowCallout",
        FAutoShapeType::QuadArrowCallout => "quadArrowCallout",
        FAutoShapeType::CircularArrow => "circularArrow",
        FAutoShapeType::FlowChartProcess => "flowChartProcess",
        FAutoShapeType::FlowChartDecision => "flowChartDecision",
        FAutoShapeType::FlowChartTerminator => "flowChartTerminator",
        FAutoShapeType::FlowChartConnector => "flowChartConnector",
        FAutoShapeType::FlowChartManualInput => "flowChartManualInput",
        FAutoShapeType::FlowChartManualOperation => "flowChartManualOperation",
        FAutoShapeType::FlowChartPreparation => "flowChartPreparation",
        FAutoShapeType::FlowChartData => "flowChartData",
        FAutoShapeType::FlowChartInternalStorage => "flowChartInternalStorage",
        FAutoShapeType::FlowChartDocument => "flowChartDocument",
        FAutoShapeType::FlowChartMultidocument => "flowChartMultidocument",
        FAutoShapeType::FlowChartPredefinedProcess => "flowChartPredefinedProcess",
        FAutoShapeType::FlowChartStoredData => "flowChartStoredData",
        FAutoShapeType::FlowChartSequentialAccessStorage => "flowChartSequentialAccessStorage",
        FAutoShapeType::FlowChartMagneticDisk => "flowChartMagneticDisk",
        FAutoShapeType::FlowChartDirectAccessStorage => "flowChartDirectAccessStorage",
        FAutoShapeType::FlowChartDisplay => "flowChartDisplay",
        FAutoShapeType::MathPlus => "mathPlus",
        FAutoShapeType::MathMinus => "mathMinus",
        FAutoShapeType::MathMultiply => "mathMultiply",
        FAutoShapeType::MathDivide => "mathDivide",
        FAutoShapeType::MathEqual => "mathEqual",
        FAutoShapeType::MathNotEqual => "mathNotEqual",
        FAutoShapeType::Gear6 => "gear6",
        FAutoShapeType::Gear9 => "gear9",
        FAutoShapeType::Funnel => "funnel",
        FAutoShapeType::ChartPlus => "chartPlus",
        FAutoShapeType::ChartStar => "chartStar",
        FAutoShapeType::ChartX => "chartX",
    }
}

/// Serializes a picture.
/// Element order per CT_Picture: nvPicPr, blipFill, spPr, style
fn serialize_picture(
    pic: &FPicture,
    slide_images: &HashMap<String, SlideImageRef>,
) -> Option<String> {
    let img = slide_images.get(&pic.relationship_id)?;

    let mut out = String::new();
    out.push_str("<p:pic>");
    out.push_str(&format!(
        "<p:nvPicPr><p:cNvPr id=\"{}\" name=\"{}\"{} /><p:cNvPicPr/><p:nvPr/></p:nvPicPr>",
        pic.id,
        encode_xml_text(&pic.name),
        pic.alt_text
            .as_ref()
            .map(|t| format!(" descr=\"{}\"", encode_xml_text(t)))
            .unwrap_or_default()
    ));
    // blipFill must come BEFORE spPr per CT_Picture sequence
    out.push_str(&format!(
        "<p:blipFill><a:blip r:embed=\"{}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>",
        encode_xml_text(&img.new_rid)
    ));
    // spPr comes after blipFill
    out.push_str("<p:spPr>");
    out.push_str(&serialize_xfrm(&pic.properties));
    out.push_str("<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>");
    out.push_str("</p:spPr>");
    out.push_str("</p:pic>");
    Some(out)
}

/// Serializes a group shape.
fn serialize_group(grp: &FGroupShape, slide_images: &HashMap<String, SlideImageRef>) -> String {
    let mut out = String::new();
    out.push_str("<p:grpSp>");
    out.push_str(&format!(
        "<p:nvGrpSpPr><p:cNvPr id=\"{}\" name=\"{}\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>",
        grp.id,
        encode_xml_text(&grp.name)
    ));
    out.push_str("<p:grpSpPr><a:xfrm");
    if let Some(rot) = grp.properties.rotation {
        out.push_str(&format!(" rot=\"{}\"", rot));
    }
    if grp.properties.flip_horizontal {
        out.push_str(" flipH=\"1\"");
    }
    if grp.properties.flip_vertical {
        out.push_str(" flipV=\"1\"");
    }
    out.push_str(&format!(
        "><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/><a:chOff x=\"{}\" y=\"{}\"/><a:chExt cx=\"{}\" cy=\"{}\"/></a:xfrm></p:grpSpPr>",
        grp.properties.x,
        grp.properties.y,
        grp.properties.width,
        grp.properties.height,
        grp.properties.x,
        grp.properties.y,
        grp.properties.width,
        grp.properties.height
    ));
    for shape in &grp.shapes {
        if let Some(xml) = serialize_shape(shape, slide_images) {
            out.push_str(&xml);
        }
    }
    out.push_str("</p:grpSp>");
    out
}

/// Serializes a connector.
fn serialize_connector(cxn: &FConnector) -> String {
    let mut out = String::new();
    out.push_str("<p:cxnSp>");
    out.push_str(&format!(
        "<p:nvCxnSpPr><p:cNvPr id=\"{}\" name=\"{}\"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>",
        cxn.id,
        encode_xml_text(&cxn.name)
    ));
    out.push_str("<p:spPr>");
    out.push_str(&serialize_xfrm(&cxn.properties));
    out.push_str(&format!(
        "<a:prstGeom prst=\"{}\"><a:avLst/></a:prstGeom>",
        if cxn.connector_type.as_deref() == Some("straight") {
            "line"
        } else {
            cxn.connector_type.as_deref().unwrap_or("line")
        }
    ));
    if let Some(color) = &cxn.line_color {
        let width = cxn.line_width.unwrap_or(9525);
        out.push_str(&format!(
            "<a:ln w=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            width,
            encode_xml_text(color)
        ));
        if let Some(start) = &cxn.arrow_start {
            out.push_str(&format!("<a:headEnd type=\"{}\"/>", encode_xml_text(start)));
        }
        if let Some(end) = &cxn.arrow_end {
            out.push_str(&format!("<a:tailEnd type=\"{}\"/>", encode_xml_text(end)));
        }
        out.push_str("</a:ln>");
    }
    out.push_str("</p:spPr>");
    out.push_str("</p:cxnSp>");
    out
}

/// Serializes a graphic frame.
/// graphicFrame uses p:xfrm (not a:xfrm) per CT_GraphicalObjectFrame
fn serialize_graphic_frame(gf: &FGraphicFrame) -> Option<String> {
    let (uri, body) = if let Some(table) = &gf.table {
        (
            "http://schemas.openxmlformats.org/drawingml/2006/table",
            serialize_table(table),
        )
    } else if let Some(rid) = &gf.chart_relationship_id {
        (
            "http://schemas.openxmlformats.org/drawingml/2006/chart",
            format!(
                "<c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" r:id=\"{}\"/>",
                encode_xml_text(rid)
            ),
        )
    } else {
        return None;
    };

    let mut out = String::new();
    out.push_str("<p:graphicFrame>");
    out.push_str(&format!(
        "<p:nvGraphicFramePr><p:cNvPr id=\"{}\" name=\"{}\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>",
        gf.id,
        encode_xml_text(&gf.name)
    ));
    // graphicFrame uses p:xfrm directly (not wrapped in spPr)
    out.push_str(&serialize_p_xfrm(&gf.properties));
    out.push_str(&format!(
        "<a:graphic><a:graphicData uri=\"{}\">{}</a:graphicData></a:graphic>",
        uri, body
    ));
    out.push_str("</p:graphicFrame>");
    Some(out)
}

/// Serializes a placeholder.
fn serialize_placeholder(ph: &FPlaceholder) -> String {
    let mut out = String::new();
    out.push_str("<p:sp>");
    out.push_str(&format!(
        "<p:nvSpPr><p:cNvPr id=\"{}\" name=\"{}\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"{}\"{} /></p:nvPr></p:nvSpPr>",
        ph.id,
        encode_xml_text(&ph.name),
        placeholder_type_to_str(ph.placeholder_type),
        ph.index
            .map(|i| format!(" idx=\"{}\"", i))
            .unwrap_or_default()
    ));
    out.push_str(&serialize_shape_properties(&ph.properties));
    out.push_str(&serialize_text_body(&ph.text_body));
    out.push_str("</p:sp>");
    out
}

/// Converts placeholder type to string.
fn placeholder_type_to_str(t: FPlaceholderType) -> &'static str {
    match t {
        FPlaceholderType::Title => "title",
        FPlaceholderType::Body => "body",
        FPlaceholderType::CenterTitle => "ctrTitle",
        FPlaceholderType::SubTitle => "subTitle",
        FPlaceholderType::Date => "dt",
        FPlaceholderType::Footer => "ftr",
        FPlaceholderType::Header => "hdr",
        FPlaceholderType::SlideNumber => "sldNum",
        FPlaceholderType::Picture => "pic",
        FPlaceholderType::Chart => "chart",
        FPlaceholderType::Table => "tbl",
        FPlaceholderType::Media => "media",
        FPlaceholderType::Object => "obj",
        FPlaceholderType::None => "",
    }
}

/// Serializes a text body.
fn serialize_text_body(body: &FTextBody) -> String {
    let mut out = String::new();
    out.push_str("<p:txBody>");
    out.push_str(&serialize_text_body_properties(&body.body_properties));
    if body.paragraphs.is_empty() {
        out.push_str("<a:p><a:endParaRPr lang=\"en-US\"/></a:p>");
    } else {
        for para in &body.paragraphs {
            out.push_str(&serialize_text_paragraph(para));
        }
    }
    out.push_str("</p:txBody>");
    out
}

/// Serializes text body properties.
fn serialize_text_body_properties(props: &FTextBodyProperties) -> String {
    let mut out = String::from("<a:bodyPr");
    if let Some(wrap) = props.wrap {
        out.push_str(&format!(
            " wrap=\"{}\"",
            if wrap { "square" } else { "none" }
        ));
    }
    if let Some(anchor) = &props.anchor {
        out.push_str(&format!(" anchor=\"{}\"", encode_xml_text(anchor)));
    }
    if let Some(l) = props.inset_left {
        out.push_str(&format!(" lIns=\"{}\"", l));
    }
    if let Some(t) = props.inset_top {
        out.push_str(&format!(" tIns=\"{}\"", t));
    }
    if let Some(r) = props.inset_right {
        out.push_str(&format!(" rIns=\"{}\"", r));
    }
    if let Some(b) = props.inset_bottom {
        out.push_str(&format!(" bIns=\"{}\"", b));
    }
    if let Some(cols) = props.num_columns {
        out.push_str(&format!(" numCol=\"{}\"", cols));
    }
    if let Some(rot) = props.rotation {
        out.push_str(&format!(" rot=\"{}\"", rot));
    }
    out.push_str("/>");
    out
}

/// Serializes a text paragraph.
fn serialize_text_paragraph(para: &FTextParagraph) -> String {
    let mut out = String::new();
    let has_ppr = para.alignment.is_some()
        || para.indent_level.is_some()
        || para.line_spacing.is_some()
        || para.space_before.is_some()
        || para.space_after.is_some()
        || para.bullet_char.is_some();

    out.push_str("<a:p>");
    if has_ppr {
        let mut ppr_attrs = String::new();
        if let Some(algn) = para.alignment {
            let algn_str = match algn {
                FParagraphAlignment::Left => "l",
                FParagraphAlignment::Center => "ctr",
                FParagraphAlignment::Right => "r",
                FParagraphAlignment::Justify => "just",
                FParagraphAlignment::JustifyLow => "justLow",
                FParagraphAlignment::Distributed => "dist",
                FParagraphAlignment::ThaiDistributed => "thaiDist",
            };
            ppr_attrs.push_str(&format!(" algn=\"{}\"", algn_str));
        }
        if let Some(lvl) = para.indent_level {
            ppr_attrs.push_str(&format!(" lvl=\"{}\"", lvl));
        }

        let mut ppr_children = String::new();
        if let Some(spc) = para.line_spacing {
            ppr_children.push_str(&format!("<a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>", spc));
        }
        if let Some(spc) = para.space_before {
            ppr_children.push_str(&format!("<a:spcBef><a:spcPts val=\"{}\"/></a:spcBef>", spc));
        }
        if let Some(spc) = para.space_after {
            ppr_children.push_str(&format!("<a:spcAft><a:spcPts val=\"{}\"/></a:spcAft>", spc));
        }
        if let Some(bu) = &para.bullet_char {
            ppr_children.push_str(&format!(
                "<a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"{}\"/>",
                encode_xml_text(bu)
            ));
        }

        if ppr_children.is_empty() {
            out.push_str(&format!("<a:pPr{}/>", ppr_attrs));
        } else {
            out.push_str(&format!("<a:pPr{}>{}</a:pPr>", ppr_attrs, ppr_children));
        }
    }
    for run in &para.runs {
        out.push_str(&serialize_text_run(run));
    }
    out.push_str("<a:endParaRPr lang=\"en-US\"/>");
    out.push_str("</a:p>");
    out
}

/// Serializes a text run.
fn serialize_text_run(run: &FRun) -> String {
    let mut out = String::new();
    out.push_str("<a:r>");
    out.push_str(&serialize_run_properties(&run.properties));
    out.push_str(&format!("<a:t>{}</a:t>", encode_xml_text(&run.text)));
    out.push_str("</a:r>");
    out
}

/// Serializes run properties.
fn serialize_run_properties(props: &FRunProperties) -> String {
    let has_content = props.bold.is_some()
        || props.italic.is_some()
        || props.underline.is_some()
        || props.strike.is_some()
        || props.size.is_some()
        || props.font_family.is_some()
        || props.color.is_some()
        || props.highlight.is_some()
        || props.caps.is_some()
        || props.small_caps.is_some()
        || props.baseline.is_some()
        || props.spacing.is_some()
        || props.font_family_east_asia.is_some();

    if !has_content {
        return String::new();
    }

    let mut attrs = String::new();
    if let Some(sz) = props.size {
        attrs.push_str(&format!(" sz=\"{}\"", sz));
    }
    if let Some(b) = props.bold {
        attrs.push_str(&format!(" b=\"{}\"", if b { "1" } else { "0" }));
    }
    if let Some(i) = props.italic {
        attrs.push_str(&format!(" i=\"{}\"", if i { "1" } else { "0" }));
    }
    if let Some(u) = props.underline {
        attrs.push_str(&format!(" u=\"{}\"", if u { "sng" } else { "none" }));
    }
    if let Some(s) = props.strike {
        attrs.push_str(&format!(
            " strike=\"{}\"",
            if s { "sngStrike" } else { "noStrike" }
        ));
    }
    if let Some(c) = props.caps {
        attrs.push_str(&format!(" caps=\"{}\"", if c { "1" } else { "0" }));
    }
    if let Some(sc) = props.small_caps {
        attrs.push_str(&format!(" smallCaps=\"{}\"", if sc { "1" } else { "0" }));
    }
    if let Some(bl) = props.baseline {
        attrs.push_str(&format!(" baseline=\"{}\"", bl));
    }
    if let Some(spc) = props.spacing {
        attrs.push_str(&format!(" spc=\"{}\"", spc));
    }

    let mut children = String::new();
    if let Some(font) = &props.font_family {
        children.push_str(&format!(
            "<a:latin typeface=\"{}\"/>",
            encode_xml_text(font)
        ));
    }
    if let Some(font_ea) = &props.font_family_east_asia {
        children.push_str(&format!(
            "<a:ea typeface=\"{}\"/>",
            encode_xml_text(font_ea)
        ));
    }
    if let Some(color) = &props.color {
        children.push_str(&format!(
            "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            encode_xml_text(color)
        ));
    }
    if let Some(hl) = &props.highlight {
        children.push_str(&format!(
            "<a:highlight><a:srgbClr val=\"{}\"/></a:highlight>",
            encode_xml_text(hl)
        ));
    }

    if children.is_empty() {
        format!("<a:rPr{}/>", attrs)
    } else {
        format!("<a:rPr{}>{}</a:rPr>", attrs, children)
    }
}

/// Serializes a table.
fn serialize_table(table: &FTable) -> String {
    let mut out = String::new();
    out.push_str("<a:tbl>");

    let mut tbl_pr_attrs = String::new();
    if let Some(fr) = table.first_row {
        tbl_pr_attrs.push_str(&format!(" firstRow=\"{}\"", if fr { "1" } else { "0" }));
    }
    if let Some(br) = table.banded_rows {
        tbl_pr_attrs.push_str(&format!(" bandRow=\"{}\"", if br { "1" } else { "0" }));
    }

    if let Some(style_id) = &table.style_id {
        // tableStyleId must be a GUID in format {XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}
        let guid = if is_valid_guid(style_id) {
            style_id.clone()
        } else {
            "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".to_string()
        };
        out.push_str(&format!(
            "<a:tblPr{}><a:tableStyleId>{}</a:tableStyleId></a:tblPr>",
            tbl_pr_attrs,
            encode_xml_text(&guid)
        ));
    } else {
        out.push_str(&format!("<a:tblPr{}/>", tbl_pr_attrs));
    }

    out.push_str("<a:tblGrid>");
    for width in &table.column_widths {
        out.push_str(&format!("<a:gridCol w=\"{}\"/>", width));
    }
    out.push_str("</a:tblGrid>");

    for row in &table.rows {
        out.push_str(&serialize_table_row(row));
    }

    out.push_str("</a:tbl>");
    out
}

/// Checks if a string is a valid GUID in the format {XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}.
fn is_valid_guid(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 38 {
        return false;
    }
    if !s.starts_with('{') || !s.ends_with('}') {
        return false;
    }
    let inner = &s[1..37];
    let parts: Vec<&str> = inner.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let lengths = [8, 4, 4, 4, 12];
    for (i, part) in parts.iter().enumerate() {
        if part.len() != lengths[i] {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
}

/// Serializes a table row.
fn serialize_table_row(row: &FTableRow) -> String {
    let mut out = String::new();
    out.push_str("<a:tr");
    let h = row.height.unwrap_or(370840);
    out.push_str(&format!(" h=\"{}\"", h));
    out.push('>');
    for cell in &row.cells {
        out.push_str(&serialize_table_cell(cell));
    }
    out.push_str("</a:tr>");
    out
}

/// Serializes a table cell.
fn serialize_table_cell(cell: &crate::core::table::FTableCell) -> String {
    let mut out = String::new();
    out.push_str("<a:tc>");

    out.push_str("<a:txBody><a:bodyPr/><a:lstStyle/>");
    if cell.paragraphs.is_empty() {
        out.push_str("<a:p><a:endParaRPr lang=\"en-US\"/></a:p>");
    } else {
        for para in &cell.paragraphs {
            out.push_str(&serialize_text_paragraph(para));
        }
    }
    out.push_str("</a:txBody>");

    let mut tcpr_attrs = String::new();
    if cell.row_span > 1 {
        tcpr_attrs.push_str(&format!(" rowSpan=\"{}\"", cell.row_span));
    }
    if cell.column_span > 1 {
        tcpr_attrs.push_str(&format!(" gridSpan=\"{}\"", cell.column_span));
    }
    if let Some(anchor) = &cell.vertical_anchor {
        tcpr_attrs.push_str(&format!(" anchor=\"{}\"", encode_xml_text(anchor)));
    }

    let mut tcpr_children = String::new();
    if let Some(fill) = &cell.fill_color {
        tcpr_children.push_str(&format!(
            "<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>",
            encode_xml_text(fill)
        ));
    }

    if tcpr_children.is_empty() {
        out.push_str(&format!("<a:tcPr{}/>", tcpr_attrs));
    } else {
        out.push_str(&format!("<a:tcPr{}>{}</a:tcPr>", tcpr_attrs, tcpr_children));
    }

    out.push_str("</a:tc>");
    out
}

/// Serializes theme.
fn serialize_theme(theme: &crate::core::theme::FTheme) -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"");
    if let Some(name) = &theme.name {
        out.push_str(&format!(" name=\"{}\"", encode_xml_text(name)));
    }
    out.push_str("><a:themeElements>");

    out.push_str("<a:clrScheme");
    if let Some(name) = &theme.color_scheme.name {
        out.push_str(&format!(" name=\"{}\"", encode_xml_text(name)));
    }
    out.push('>');
    let colors = [
        ("a:dk1", &theme.color_scheme.dark1),
        ("a:lt1", &theme.color_scheme.light1),
        ("a:dk2", &theme.color_scheme.dark2),
        ("a:lt2", &theme.color_scheme.light2),
        ("a:accent1", &theme.color_scheme.accent1),
        ("a:accent2", &theme.color_scheme.accent2),
        ("a:accent3", &theme.color_scheme.accent3),
        ("a:accent4", &theme.color_scheme.accent4),
        ("a:accent5", &theme.color_scheme.accent5),
        ("a:accent6", &theme.color_scheme.accent6),
        ("a:hlink", &theme.color_scheme.hyperlink),
        ("a:folHlink", &theme.color_scheme.followed_hyperlink),
    ];
    for (tag, color) in colors {
        let val = color.clone().unwrap_or_else(|| "000000".to_string());
        out.push_str(&format!(
            "<{}><a:srgbClr val=\"{}\"/></{}>",
            tag,
            encode_xml_text(&val),
            tag
        ));
    }
    out.push_str("</a:clrScheme>");

    out.push_str("<a:fontScheme");
    if let Some(name) = &theme.font_scheme.name {
        out.push_str(&format!(" name=\"{}\"", encode_xml_text(name)));
    }
    out.push_str("><a:majorFont>");
    if let Some(f) = &theme.font_scheme.major_latin {
        out.push_str(&format!("<a:latin typeface=\"{}\"/>", encode_xml_text(f)));
    }
    if let Some(f) = &theme.font_scheme.major_east_asia {
        out.push_str(&format!("<a:ea typeface=\"{}\"/>", encode_xml_text(f)));
    }
    out.push_str("<a:cs typeface=\"\"/>");
    out.push_str("</a:majorFont><a:minorFont>");
    if let Some(f) = &theme.font_scheme.minor_latin {
        out.push_str(&format!("<a:latin typeface=\"{}\"/>", encode_xml_text(f)));
    }
    if let Some(f) = &theme.font_scheme.minor_east_asia {
        out.push_str(&format!("<a:ea typeface=\"{}\"/>", encode_xml_text(f)));
    }
    out.push_str("<a:cs typeface=\"\"/>");
    out.push_str("</a:minorFont></a:fontScheme>");

    out.push_str("<a:fmtScheme name=\"Office\">");
    out.push_str("<a:fillStyleLst>");
    out.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    out.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    out.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    out.push_str("</a:fillStyleLst>");
    out.push_str("<a:lnStyleLst>");
    out.push_str("<a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>");
    out.push_str(
        "<a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>",
    );
    out.push_str(
        "<a:ln w=\"19050\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>",
    );
    out.push_str("</a:lnStyleLst>");
    out.push_str("<a:effectStyleLst>");
    out.push_str("<a:effectStyle><a:effectLst/></a:effectStyle>");
    out.push_str("<a:effectStyle><a:effectLst/></a:effectStyle>");
    out.push_str("<a:effectStyle><a:effectLst/></a:effectStyle>");
    out.push_str("</a:effectStyleLst>");
    out.push_str("<a:bgFillStyleLst>");
    out.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    out.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    out.push_str("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>");
    out.push_str("</a:bgFillStyleLst>");
    out.push_str("</a:fmtScheme>");

    out.push_str("</a:themeElements></a:theme>");
    out
}

/// Serializes a minimal slide master (required by OOXML spec).
fn serialize_slide_master() -> String {
    println!("[DEBUG] serialize_slide_master: starting");
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<p:sldMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">");
    out.push_str("<p:cSld>");
    out.push_str("<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>");
    out.push_str("<p:spTree>");
    out.push_str("<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>");
    out.push_str("<p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>");
    // Title placeholder
    out.push_str("<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title Placeholder 1\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr>");
    out.push_str("<p:spPr><a:xfrm><a:off x=\"457200\" y=\"274638\"/><a:ext cx=\"8229600\" cy=\"1143000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
    out.push_str("<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\"/><a:t>Click to edit Master title style</a:t></a:r><a:endParaRPr lang=\"en-US\"/></a:p></p:txBody></p:sp>");
    // Body placeholder
    out.push_str("<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Text Placeholder 2\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr>");
    out.push_str("<p:spPr><a:xfrm><a:off x=\"457200\" y=\"1600200\"/><a:ext cx=\"8229600\" cy=\"4525963\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
    out.push_str("<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr lvl=\"0\"/><a:r><a:rPr lang=\"en-US\"/><a:t>Click to edit Master text styles</a:t></a:r></a:p></p:txBody></p:sp>");
    // Date placeholder
    out.push_str("<p:sp><p:nvSpPr><p:cNvPr id=\"4\" name=\"Date Placeholder 3\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"dt\" sz=\"half\" idx=\"2\"/></p:nvPr></p:nvSpPr>");
    out.push_str("<p:spPr><a:xfrm><a:off x=\"457200\" y=\"6350000\"/><a:ext cx=\"2133600\" cy=\"365125\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
    out.push_str("<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:fld id=\"{1D8BD707-D9CF-40AE-B4C6-C98DA3205C09}\" type=\"datetimeFigureOut\"><a:rPr lang=\"en-US\"/><a:t>1/27/13</a:t></a:fld><a:endParaRPr lang=\"en-US\"/></a:p></p:txBody></p:sp>");
    // Footer placeholder
    out.push_str("<p:sp><p:nvSpPr><p:cNvPr id=\"5\" name=\"Footer Placeholder 4\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"ftr\" sz=\"quarter\" idx=\"3\"/></p:nvPr></p:nvSpPr>");
    out.push_str("<p:spPr><a:xfrm><a:off x=\"3124200\" y=\"6350000\"/><a:ext cx=\"2895600\" cy=\"365125\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
    out.push_str("<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-US\"/></a:p></p:txBody></p:sp>");
    // Slide number placeholder
    out.push_str("<p:sp><p:nvSpPr><p:cNvPr id=\"6\" name=\"Slide Number Placeholder 5\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"sldNum\" sz=\"quarter\" idx=\"4\"/></p:nvPr></p:nvSpPr>");
    out.push_str("<p:spPr><a:xfrm><a:off x=\"6553200\" y=\"6350000\"/><a:ext cx=\"2133600\" cy=\"365125\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
    out.push_str("<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:fld id=\"{B6F15528-21DE-4FAA-801E-634DDDA7AAD2}\" type=\"slidenum\"><a:rPr lang=\"en-US\"/><a:t>#</a:t></a:fld><a:endParaRPr lang=\"en-US\"/></a:p></p:txBody></p:sp>");
    out.push_str("</p:spTree>");
    out.push_str("</p:cSld>");
    out.push_str("<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>");
    out.push_str("<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst>");
    out.push_str("<p:txStyles>");
    out.push_str("<p:titleStyle><a:lvl1pPr><a:defRPr sz=\"4400\" b=\"1\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mj-lt\"/><a:ea typeface=\"+mj-ea\"/><a:cs typeface=\"+mj-cs\"/></a:defRPr></a:lvl1pPr></p:titleStyle>");
    out.push_str("<p:bodyStyle><a:lvl1pPr><a:defRPr sz=\"3200\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl1pPr><a:lvl2pPr><a:defRPr sz=\"2800\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl2pPr><a:lvl3pPr><a:defRPr sz=\"2400\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl3pPr></p:bodyStyle>");
    out.push_str("<p:otherStyle><a:lvl1pPr><a:defRPr sz=\"1800\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl1pPr></p:otherStyle>");
    out.push_str("</p:txStyles>");
    out.push_str("</p:sldMaster>");
    println!("[DEBUG] serialize_slide_master: finished, length: {}", out.len());
    out
}

/// Serializes slide master relationships.
fn serialize_slide_master_rels(has_theme: bool) -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    // Link to slide layout
    out.push_str(
        "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>",
    );
    // Link to theme
    if has_theme {
        out.push_str(
            "<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/>",
        );
    }
    out.push_str("</Relationships>");
    out
}

/// Serializes a minimal slide layout (required by OOXML spec).
fn serialize_slide_layout() -> String {
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str("<p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" type=\"blank\" preserve=\"1\">");
    out.push_str("<p:cSld name=\"Blank\">");
    out.push_str("<p:spTree>");
    out.push_str("<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>");
    out.push_str("<p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>");
    out.push_str("</p:spTree>");
    out.push_str("</p:cSld>");
    out.push_str("<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>");
    out.push_str("</p:sldLayout>");
    out
}

/// Serializes slide layout relationships.
fn serialize_slide_layout_rels() -> String {
    println!("[DEBUG] serialize_slide_layout_rels: starting");
    let mut out = String::new();
    out.push_str(XML_DECL);
    out.push_str(
        "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    // Link to slide master
    out.push_str(
        "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/>",
    );
    out.push_str("</Relationships>");
    println!("[DEBUG] serialize_slide_layout_rels: finished, length: {}", out.len());
    out
}