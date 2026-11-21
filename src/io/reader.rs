//! Presentation parsing logic.
use crate::core::media::{FImage, FMedia, FMediaType};
use crate::core::presentation::{FPresentation, FSlideId, FSlideSize};
use crate::core::shape::*;
use crate::core::slide::{FBackground, FBackgroundType, FSlide, FSlideLayout, FSlideTransition};
use crate::core::table::*;
use crate::core::text::*;
use crate::core::theme::{FColorScheme, FFontScheme, FTheme};
use crate::{
    FPackagePart, FPptError, collect_elements, decode_xml_text, element_exists,
    extract_element_inner, extract_element_with_span, extract_first_attr, extract_first_text,
    flag_present,
};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::read::ZipArchive;
/// Parses a complete presentation from raw bytes.
pub(crate) fn parse_presentation(data: &[u8]) -> crate::FPptResult<FPresentation> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;
    let mut pres = FPresentation::default();
    let mut parts: Vec<FPackagePart> = Vec::new();
    let mut slide_rels: HashMap<String, String> = HashMap::new();
    let mut media_map: HashMap<String, FMediaType> = HashMap::new();
    let mut pres_rels_xml: Option<String> = None;
    // First pass: collect all parts and identify slides.
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().unwrap_or_default().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        match name.as_str() {
            "ppt/presentation.xml" => {
                parse_presentation_xml(&buf, &mut pres)?;
            }
            "ppt/_rels/presentation.xml.rels" => {
                pres_rels_xml = Some(String::from_utf8_lossy(&buf).to_string());
            }
            s if s.starts_with("ppt/slides/slide") && s.ends_with(".xml") => {
                parts.push(FPackagePart {
                    path: name.clone(),
                    data: buf,
                });
            }
            s if s.starts_with("ppt/slides/_rels/") && s.ends_with(".rels") => {
                if let Ok(text) = std::str::from_utf8(&buf) {
                    parse_slide_rels(text, &mut slide_rels, &mut media_map);
                }
                parts.push(FPackagePart {
                    path: name.clone(),
                    data: buf,
                });
            }
            s if s.starts_with("ppt/media/") => {
                let media_type = detect_media_type(&name);
                media_map.insert(name.clone(), media_type);
                parts.push(FPackagePart {
                    path: name.clone(),
                    data: buf,
                });
            }
            "ppt/theme/theme1.xml" => {
                pres.theme = parse_theme(&buf)?;
                parts.push(FPackagePart {
                    path: name.clone(),
                    data: buf,
                });
            }
            _ => {
                parts.push(FPackagePart {
                    path: name,
                    data: buf,
                });
            }
        }
    }
    // Resolve slide order from presentation.xml.rels.
    if let Some(rels_xml) = &pres_rels_xml {
        for (rid, target) in parse_presentation_rels(rels_xml) {
            if let Some(slide) = pres.slides.iter_mut().find(|s| s.relationship_id == rid) {
                slide.slide_number = extract_slide_number(&target).unwrap_or(0);
            }
        }
    }
    // Load slide contents.
    let mut slides: Vec<FSlideId> = Vec::new();
    for part in &parts {
        if part.path.starts_with("ppt/slides/slide") && part.path.ends_with(".xml") {
            let slide = parse_slide(&part.data, &slide_rels)?;
            let slide_num = extract_slide_number(&part.path).unwrap_or(0);
            let rid = format!("rId{}", slide_num);
            slides.push(FSlideId {
                relationship_id: rid,
                slide_number: slide_num,
                slide: Some(Box::new(slide)),
            });
        }
    }
    slides.sort_by_key(|s| s.slide_number);
    pres.slides = slides;
    pres.package.main_part = "ppt/presentation.xml".to_string();
    pres.package.parts = parts
        .into_iter()
        .filter(|p| !p.path.starts_with("ppt/slides/slide") || !p.path.ends_with(".xml"))
        .collect();
    pres.dirty = false;
    Ok(pres)
}
/// Parses presentation.xml for slide size and slide ID list.
fn parse_presentation_xml(xml: &[u8], pres: &mut FPresentation) -> crate::FPptResult<()> {
    let text = std::str::from_utf8(xml)
        .map_err(|e| FPptError::Xml(format!("presentation.xml is not valid utf-8: {}", e)))?;
    if let Some(sld_sz) = extract_element_inner(text, "p:sldSz") {
        let width = extract_first_attr(&sld_sz, "p:sldSz", "cx")
            .and_then(|v| v.parse().ok())
            .unwrap_or(9144000);
        let height = extract_first_attr(&sld_sz, "p:sldSz", "cy")
            .and_then(|v| v.parse().ok())
            .unwrap_or(6858000);
        let size_type = extract_first_attr(&sld_sz, "p:sldSz", "type");
        pres.slide_size = FSlideSize {
            width,
            height,
            size_type,
        };
    }
    if let Some(sld_id_lst) = extract_element_inner(text, "p:sldIdLst") {
        for sld_id_xml in collect_elements(&sld_id_lst, "p:sldId") {
            let rid = extract_first_attr(&sld_id_xml, "p:sldId", "r:id").unwrap_or_default();
            let num = extract_first_attr(&sld_id_xml, "p:sldId", "id")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            pres.slides.push(FSlideId {
                relationship_id: rid,
                slide_number: num,
                slide: None,
            });
        }
    }
    Ok(())
}
/// Parses presentation.xml.rels for slide targets.
fn parse_presentation_rels(xml: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for rel in collect_elements(xml, "Relationship") {
        let id = extract_first_attr(&rel, "Relationship", "Id").unwrap_or_default();
        let target = extract_first_attr(&rel, "Relationship", "Target").unwrap_or_default();
        result.push((id, target));
    }
    result
}
/// Parses slide relationship file for media references.
fn parse_slide_rels(
    xml: &str,
    slide_rels: &mut HashMap<String, String>,
    media_map: &mut HashMap<String, FMediaType>,
) {
    for rel in collect_elements(xml, "Relationship") {
        let id = extract_first_attr(&rel, "Relationship", "Id").unwrap_or_default();
        let target = extract_first_attr(&rel, "Relationship", "Target").unwrap_or_default();
        let rel_type = extract_first_attr(&rel, "Relationship", "Type").unwrap_or_default();
        if rel_type.contains("image") || rel_type.contains("video") || rel_type.contains("media") {
            slide_rels.insert(id.clone(), target.clone());
            let media_type = detect_media_type(&target);
            media_map.insert(target, media_type);
        }
    }
}
/// Detects media type from file extension.
fn detect_media_type(path: &str) -> FMediaType {
    let lower = path.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        FMediaType::Jpeg
    } else if lower.ends_with(".png") {
        FMediaType::Png
    } else if lower.ends_with(".gif") {
        FMediaType::Gif
    } else if lower.ends_with(".bmp") {
        FMediaType::Bmp
    } else if lower.ends_with(".tiff") || lower.ends_with(".tif") {
        FMediaType::Tiff
    } else if lower.ends_with(".svg") {
        FMediaType::Svg
    } else if lower.ends_with(".mp4") {
        FMediaType::Mp4
    } else {
        FMediaType::Unknown
    }
}
/// Extracts slide number from path like ppt/slides/slide3.xml.
fn extract_slide_number(path: &str) -> Option<u32> {
    let stem = path.strip_prefix("ppt/slides/slide")?;
    let num_str: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}
/// Parses a single slide part.
fn parse_slide(xml: &[u8], slide_rels: &HashMap<String, String>) -> crate::FPptResult<FSlide> {
    let text = std::str::from_utf8(xml)
        .map_err(|e| FPptError::Xml(format!("slide is not valid utf-8: {}", e)))?;
    let mut slide = FSlide::default();
    if let Some(bg_xml) = extract_element_inner(text, "p:bg") {
        slide.background = Some(parse_background(&bg_xml));
    }
    if let Some(trans_xml) = extract_element_inner(text, "p:transition") {
        slide.transition = Some(parse_transition(&trans_xml));
    }
    if let Some(sp_tree) = extract_element_inner(text, "p:spTree") {
        slide.shapes = parse_shape_tree(&sp_tree, slide_rels)?;
    }
    slide.raw_xml = Some(text.to_string());
    Ok(slide)
}
/// Parses slide background.
fn parse_background(xml: &str) -> FBackground {
    let mut bg = FBackground::default();
    if let Some(bgpr) = extract_element_inner(xml, "p:bgPr") {
        if let Some(solid) = extract_element_inner(&bgpr, "a:solidFill") {
            bg.fill_type = Some(FBackgroundType::Solid);
            bg.color = extract_first_attr(&solid, "a:srgbClr", "val");
        } else if extract_element_inner(&bgpr, "a:gradFill").is_some() {
            bg.fill_type = Some(FBackgroundType::Gradient);
        } else if extract_element_inner(&bgpr, "a:blipFill").is_some() {
            bg.fill_type = Some(FBackgroundType::Picture);
        }
    }
    bg.raw_xml = Some(xml.to_string());
    bg
}
/// Parses slide transition.
fn parse_transition(xml: &str) -> FSlideTransition {
    let mut trans = FSlideTransition::default();
    if let Some(fade) = extract_element_inner(xml, "p:fade") {
        trans.transition_type = Some("fade".to_string());
        if let Some(dur) = extract_first_attr(&fade, "p:fade", "dur") {
            trans.duration_ms = dur.parse().ok();
        }
    } else if let Some(push) = extract_element_inner(xml, "p:push") {
        trans.transition_type = Some("push".to_string());
        if let Some(dur) = extract_first_attr(&push, "p:push", "dur") {
            trans.duration_ms = dur.parse().ok();
        }
    } else if let Some(wipe) = extract_element_inner(xml, "p:wipe") {
        trans.transition_type = Some("wipe".to_string());
        if let Some(dur) = extract_first_attr(&wipe, "p:wipe", "dur") {
            trans.duration_ms = dur.parse().ok();
        }
    }
    if let Some(adv) = extract_first_attr(xml, "p:transition", "advClick") {
        trans.advance_on_click = Some(adv == "1" || adv == "true");
    }
    if let Some(adv_tm) = extract_first_attr(xml, "p:transition", "advTm") {
        trans.advance_after_ms = adv_tm.parse().ok();
    }
    trans.raw_xml = Some(xml.to_string());
    trans
}
/// Parses the shape tree of a slide.
fn parse_shape_tree(
    xml: &str,
    slide_rels: &HashMap<String, String>,
) -> crate::FPptResult<Vec<FShape>> {
    let mut shapes = Vec::new();
    let mut cursor = 0usize;
    while cursor < xml.len() {
        let rest = &xml[cursor..];
        // Find next shape element.
        let sp_pos = rest.find("<p:sp>").map(|v| (v, "p:sp"));
        let pic_pos = rest.find("<p:pic>").map(|v| (v, "p:pic"));
        let grp_pos = rest.find("<p:grpSp>").map(|v| (v, "p:grpSp"));
        let cxn_pos = rest.find("<p:cxnSp>").map(|v| (v, "p:cxnSp"));
        let gf_pos = rest.find("<p:graphicFrame>").map(|v| (v, "p:graphicFrame"));
        let next = [sp_pos, pic_pos, grp_pos, cxn_pos, gf_pos]
            .into_iter()
            .flatten()
            .min_by_key(|(pos, _)| *pos);
        let (offset, kind) = match next {
            Some(v) => v,
            None => break,
        };
        cursor += offset;
        let (inner, consumed) = match extract_element_with_span(&xml[cursor..], kind) {
            Some(v) => v,
            None => break,
        };
        cursor += consumed;
        match kind {
            "p:sp" => {
                if let Some(shape) = parse_shape(&inner, slide_rels) {
                    shapes.push(shape);
                }
            }
            "p:pic" => {
                if let Some(shape) = parse_picture(&inner, slide_rels) {
                    shapes.push(shape);
                }
            }
            "p:grpSp" => {
                if let Some(shape) = parse_group(&inner, slide_rels) {
                    shapes.push(shape);
                }
            }
            "p:cxnSp" => {
                if let Some(shape) = parse_connector(&inner, slide_rels) {
                    shapes.push(shape);
                }
            }
            "p:graphicFrame" => {
                if let Some(shape) = parse_graphic_frame(&inner, slide_rels) {
                    shapes.push(shape);
                }
            }
            _ => {}
        }
    }
    Ok(shapes)
}
/// Parses a shape element.
fn parse_shape(xml: &str, slide_rels: &HashMap<String, String>) -> Option<FShape> {
    let nv_sp_pr = extract_element_inner(xml, "p:nvSpPr")?;
    let c_nv_pr = extract_element_inner(&nv_sp_pr, "p:cNvPr")?;
    let id = extract_first_attr(&c_nv_pr, "p:cNvPr", "id")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let name = extract_first_attr(&c_nv_pr, "p:cNvPr", "name").unwrap_or_default();
    let properties = parse_shape_properties(xml)?;
    let text_body = parse_text_body(xml)?;
    // Check for placeholder.
    if let Some(nv_pr) = extract_element_inner(&nv_sp_pr, "p:nvPr") {
        if let Some(ph) = extract_element_inner(&nv_pr, "p:ph") {
            let ph_type = extract_first_attr(&ph, "p:ph", "type").unwrap_or_default();
            let index = extract_first_attr(&ph, "p:ph", "idx").and_then(|v| v.parse().ok());
            return Some(FShape::Placeholder(FPlaceholder {
                id,
                name,
                properties,
                placeholder_type: match ph_type.as_str() {
                    "title" => FPlaceholderType::Title,
                    "body" => FPlaceholderType::Body,
                    "ctrTitle" => FPlaceholderType::CenterTitle,
                    "subTitle" => FPlaceholderType::SubTitle,
                    "dt" => FPlaceholderType::Date,
                    "ftr" => FPlaceholderType::Footer,
                    "hdr" => FPlaceholderType::Header,
                    "sldNum" => FPlaceholderType::SlideNumber,
                    "pic" => FPlaceholderType::Picture,
                    _ => FPlaceholderType::None,
                },
                index,
                text_body,
            }));
        }
    }
    // Check for auto shape with preset geometry.
    if let Some(sp_pr) = extract_element_inner(xml, "p:spPr") {
        if let Some(prst_geom) = extract_element_inner(&sp_pr, "a:prstGeom") {
            let shape_type = extract_first_attr(&prst_geom, "a:prstGeom", "prst")
                .map(|v| parse_auto_shape_type(&v))
                .unwrap_or_default();
            let fill_color = extract_element_inner(&sp_pr, "a:solidFill")
                .and_then(|fill| extract_first_attr(&fill, "a:srgbClr", "val"));
            let (line_color, line_width) = extract_element_inner(&sp_pr, "a:ln")
                .map(|ln| {
                    let color = extract_element_inner(&ln, "a:solidFill")
                        .and_then(|fill| extract_first_attr(&fill, "a:srgbClr", "val"));
                    let width = extract_first_attr(&ln, "a:ln", "w").and_then(|v| v.parse().ok());
                    (color, width)
                })
                .unwrap_or((None, None));
            return Some(FShape::AutoShape(FAutoShape {
                id,
                name,
                properties,
                text_body,
                shape_type,
                fill_color,
                line_color,
                line_width,
            }));
        }
    }
    Some(FShape::TextBox(FShapeContainer {
        id,
        name,
        properties,
        text_body,
    }))
}
/// Parses preset geometry type string.
fn parse_auto_shape_type(s: &str) -> FAutoShapeType {
    match s {
        "ellipse" => FAutoShapeType::Ellipse,
        "triangle" => FAutoShapeType::Triangle,
        "rtTriangle" => FAutoShapeType::RightTriangle,
        "diamond" => FAutoShapeType::Diamond,
        "pentagon" => FAutoShapeType::Pentagon,
        "hexagon" => FAutoShapeType::Hexagon,
        "octagon" => FAutoShapeType::Octagon,
        "star5" => FAutoShapeType::Star5,
        "star6" => FAutoShapeType::Star6,
        "arrow" => FAutoShapeType::Arrow,
        "line" => FAutoShapeType::Line,
        "roundRect" => FAutoShapeType::RoundRectangle,
        "snipRect" => FAutoShapeType::SnipRectangle,
        "parallelogram" => FAutoShapeType::Parallelogram,
        "trapezoid" => FAutoShapeType::Trapezoid,
        "heart" => FAutoShapeType::Heart,
        "lightningBolt" => FAutoShapeType::LightningBolt,
        "sun" => FAutoShapeType::Sun,
        "moon" => FAutoShapeType::Moon,
        "cloud" => FAutoShapeType::Cloud,
        "arc" => FAutoShapeType::Arc,
        "pie" => FAutoShapeType::Pie,
        "donut" => FAutoShapeType::Donut,
        "noSmoking" => FAutoShapeType::NoSymbol,
        "blockArc" => FAutoShapeType::BlockArc,
        "foldedCorner" => FAutoShapeType::FoldedCorner,
        "smileyFace" => FAutoShapeType::SmileyFace,
        "teardrop" => FAutoShapeType::Teardrop,
        "trapezoid2" => FAutoShapeType::Trapezoid2,
        "irregularSeal1" => FAutoShapeType::IrregularSeal1,
        "irregularSeal2" => FAutoShapeType::IrregularSeal2,
        "leftRightArrow" => FAutoShapeType::LeftRightArrow,
        "upDownArrow" => FAutoShapeType::UpDownArrow,
        "leftArrow" => FAutoShapeType::LeftArrow,
        "rightArrow" => FAutoShapeType::RightArrow,
        "upArrow" => FAutoShapeType::UpArrow,
        "downArrow" => FAutoShapeType::DownArrow,
        "bentArrow" => FAutoShapeType::BendedArrow,
        "uturnArrow" => FAutoShapeType::UTurnArrow,
        "leftUpArrow" => FAutoShapeType::LeftUpArrow,
        "bentUpArrow" => FAutoShapeType::BentUpArrow,
        "curvedRightArrow" => FAutoShapeType::CurvedRightArrow,
        "curvedLeftArrow" => FAutoShapeType::CurvedLeftArrow,
        "curvedUpArrow" => FAutoShapeType::CurvedUpArrow,
        "curvedDownArrow" => FAutoShapeType::CurvedDownArrow,
        "stripedRightArrow" => FAutoShapeType::StripedRightArrow,
        "notchedRightArrow" => FAutoShapeType::NotchedRightArrow,
        "pentagonArrow" => FAutoShapeType::PentagonArrow,
        "chevron" => FAutoShapeType::Chevron,
        "rightArrowCallout" => FAutoShapeType::RightArrowCallout,
        "downArrowCallout" => FAutoShapeType::DownArrowCallout,
        "leftArrowCallout" => FAutoShapeType::LeftArrowCallout,
        "upArrowCallout" => FAutoShapeType::UpArrowCallout,
        "leftRightArrowCallout" => FAutoShapeType::LeftRightArrowCallout,
        "upDownArrowCallout" => FAutoShapeType::UpDownArrowCallout,
        "quadArrowCallout" => FAutoShapeType::QuadArrowCallout,
        "circularArrow" => FAutoShapeType::CircularArrow,
        "flowChartProcess" => FAutoShapeType::FlowChartProcess,
        "flowChartDecision" => FAutoShapeType::FlowChartDecision,
        "flowChartTerminator" => FAutoShapeType::FlowChartTerminator,
        "flowChartConnector" => FAutoShapeType::FlowChartConnector,
        "flowChartManualInput" => FAutoShapeType::FlowChartManualInput,
        "flowChartManualOperation" => FAutoShapeType::FlowChartManualOperation,
        "flowChartPreparation" => FAutoShapeType::FlowChartPreparation,
        "flowChartData" => FAutoShapeType::FlowChartData,
        "flowChartInternalStorage" => FAutoShapeType::FlowChartInternalStorage,
        "flowChartDocument" => FAutoShapeType::FlowChartDocument,
        "flowChartMultidocument" => FAutoShapeType::FlowChartMultidocument,
        "flowChartPredefinedProcess" => FAutoShapeType::FlowChartPredefinedProcess,
        "flowChartStoredData" => FAutoShapeType::FlowChartStoredData,
        "flowChartSequentialAccessStorage" => FAutoShapeType::FlowChartSequentialAccessStorage,
        "flowChartMagneticDisk" => FAutoShapeType::FlowChartMagneticDisk,
        "flowChartDirectAccessStorage" => FAutoShapeType::FlowChartDirectAccessStorage,
        "flowChartDisplay" => FAutoShapeType::FlowChartDisplay,
        "mathPlus" => FAutoShapeType::MathPlus,
        "mathMinus" => FAutoShapeType::MathMinus,
        "mathMultiply" => FAutoShapeType::MathMultiply,
        "mathDivide" => FAutoShapeType::MathDivide,
        "mathEqual" => FAutoShapeType::MathEqual,
        "mathNotEqual" => FAutoShapeType::MathNotEqual,
        "gear6" => FAutoShapeType::Gear6,
        "gear9" => FAutoShapeType::Gear9,
        "funnel" => FAutoShapeType::Funnel,
        "chartPlus" => FAutoShapeType::ChartPlus,
        "chartStar" => FAutoShapeType::ChartStar,
        "chartX" => FAutoShapeType::ChartX,
        _ => FAutoShapeType::Rectangle,
    }
}
/// Parses a picture element.
fn parse_picture(xml: &str, slide_rels: &HashMap<String, String>) -> Option<FShape> {
    let nv_pic_pr = extract_element_inner(xml, "p:nvPicPr")?;
    let c_nv_pr = extract_element_inner(&nv_pic_pr, "p:cNvPr")?;
    let id = extract_first_attr(&c_nv_pr, "p:cNvPr", "id")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let name = extract_first_attr(&c_nv_pr, "p:cNvPr", "name").unwrap_or_default();
    let alt_text = extract_first_attr(&c_nv_pr, "p:cNvPr", "descr");
    let properties = parse_shape_properties(xml)?;
    let blip_fill = extract_element_inner(xml, "p:blipFill")?;
    let blip = extract_element_inner(&blip_fill, "a:blip")?;
    let relationship_id = extract_first_attr(&blip, "a:blip", "r:embed").unwrap_or_default();
    let target = slide_rels.get(&relationship_id).cloned();
    let content_type = target
        .as_ref()
        .map(|path| detect_media_type(path))
        .map(|mt| format!("{:?}", mt));
    Some(FShape::Picture(FPicture {
        id,
        name,
        properties,
        relationship_id,
        content_type,
        alt_text,
        target,
    }))
}
/// Parses a group shape.
fn parse_group(xml: &str, slide_rels: &HashMap<String, String>) -> Option<FShape> {
    let nv_grp_sp_pr = extract_element_inner(xml, "p:nvGrpSpPr")?;
    let c_nv_pr = extract_element_inner(&nv_grp_sp_pr, "p:cNvPr")?;
    let id = extract_first_attr(&c_nv_pr, "p:cNvPr", "id")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let name = extract_first_attr(&c_nv_pr, "p:cNvPr", "name").unwrap_or_default();
    let properties = parse_shape_properties(xml)?;
    let sp_tree = extract_element_inner(xml, "p:spTree")?;
    let shapes = parse_shape_tree(&sp_tree, slide_rels).ok()?;
    Some(FShape::Group(FGroupShape {
        id,
        name,
        shapes,
        properties,
    }))
}
/// Parses a connector element.
fn parse_connector(xml: &str, _slide_rels: &HashMap<String, String>) -> Option<FShape> {
    let nv_cxn_sp_pr = extract_element_inner(xml, "p:nvCxnSpPr")?;
    let c_nv_pr = extract_element_inner(&nv_cxn_sp_pr, "p:cNvPr")?;
    let id = extract_first_attr(&c_nv_pr, "p:cNvPr", "id")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let name = extract_first_attr(&c_nv_pr, "p:cNvPr", "name").unwrap_or_default();
    let properties = parse_shape_properties(xml)?;
    let connector_type = extract_element_inner(xml, "a:prstGeom")
        .and_then(|geom| extract_first_attr(&geom, "a:prstGeom", "prst"));
    let (line_color, line_width, arrow_start, arrow_end) = extract_element_inner(xml, "a:ln")
        .map(|ln| {
            let color = extract_element_inner(&ln, "a:solidFill")
                .and_then(|fill| extract_first_attr(&fill, "a:srgbClr", "val"));
            let width = extract_first_attr(&ln, "a:ln", "w").and_then(|v| v.parse().ok());
            let head = extract_element_inner(&ln, "a:headEnd")
                .and_then(|h| extract_first_attr(&h, "a:headEnd", "type"));
            let tail = extract_element_inner(&ln, "a:tailEnd")
                .and_then(|t| extract_first_attr(&t, "a:tailEnd", "type"));
            (color, width, head, tail)
        })
        .unwrap_or((None, None, None, None));
    Some(FShape::Connector(FConnector {
        id,
        name,
        properties,
        connector_type,
        line_color,
        line_width,
        arrow_start,
        arrow_end,
    }))
}
/// Parses a graphic frame (table or chart).
fn parse_graphic_frame(xml: &str, _slide_rels: &HashMap<String, String>) -> Option<FShape> {
    let nv_graphic_frame_pr = extract_element_inner(xml, "p:nvGraphicFramePr")?;
    let c_nv_pr = extract_element_inner(&nv_graphic_frame_pr, "p:cNvPr")?;
    let id = extract_first_attr(&c_nv_pr, "p:cNvPr", "id")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let name = extract_first_attr(&c_nv_pr, "p:cNvPr", "name").unwrap_or_default();
    let properties = parse_shape_properties(xml)?;
    let graphic = extract_element_inner(xml, "a:graphic")?;
    let graphic_data = extract_element_inner(&graphic, "a:graphicData")?;
    let table = extract_element_inner(&graphic_data, "a:tbl").map(|tbl| parse_table(&tbl));
    let chart_relationship_id = extract_element_inner(&graphic_data, "c:chart")
        .and_then(|chart| extract_first_attr(&chart, "c:chart", "r:id"));
    Some(FShape::GraphicFrame(FGraphicFrame {
        id,
        name,
        properties,
        table,
        chart_relationship_id,
    }))
}
/// Parses shape properties (position and size).
fn parse_shape_properties(xml: &str) -> Option<FShapeProperties> {
    let sp_pr = extract_element_inner(xml, "p:spPr")?;
    let xfrm = extract_element_inner(&sp_pr, "a:xfrm")?;
    let off = extract_element_inner(&xfrm, "a:off")?;
    let ext = extract_element_inner(&xfrm, "a:ext")?;
    let x = extract_first_attr(&off, "a:off", "x")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let y = extract_first_attr(&off, "a:off", "y")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let width = extract_first_attr(&ext, "a:ext", "cx")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let height = extract_first_attr(&ext, "a:ext", "cy")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let rotation = extract_first_attr(&xfrm, "a:xfrm", "rot").and_then(|v| v.parse().ok());
    let flip_horizontal = extract_first_attr(&xfrm, "a:xfrm", "flipH")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    let flip_vertical = extract_first_attr(&xfrm, "a:xfrm", "flipV")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    Some(FShapeProperties {
        x,
        y,
        width,
        height,
        rotation,
        flip_horizontal,
        flip_vertical,
    })
}
/// Parses text body of a shape.
fn parse_text_body(xml: &str) -> Option<FTextBody> {
    let tx_body = extract_element_inner(xml, "p:txBody")?;
    let mut body = FTextBody::default();
    if let Some(body_pr) = extract_element_inner(&tx_body, "a:bodyPr") {
        body.body_properties = parse_text_body_properties(&body_pr);
    }
    for p_xml in collect_elements(&tx_body, "a:p") {
        body.paragraphs.push(parse_text_paragraph(&p_xml));
    }
    Some(body)
}
/// Parses text body properties.
fn parse_text_body_properties(xml: &str) -> FTextBodyProperties {
    let mut props = FTextBodyProperties::default();
    if let Some(wrap) = extract_first_attr(xml, "a:bodyPr", "wrap") {
        props.wrap = Some(wrap == "square" || wrap == "1");
    }
    props.anchor = extract_first_attr(xml, "a:bodyPr", "anchor");
    props.inset_left = extract_first_attr(xml, "a:bodyPr", "lIns").and_then(|v| v.parse().ok());
    props.inset_top = extract_first_attr(xml, "a:bodyPr", "tIns").and_then(|v| v.parse().ok());
    props.inset_right = extract_first_attr(xml, "a:bodyPr", "rIns").and_then(|v| v.parse().ok());
    props.inset_bottom = extract_first_attr(xml, "a:bodyPr", "bIns").and_then(|v| v.parse().ok());
    props.num_columns = extract_first_attr(xml, "a:bodyPr", "numCol").and_then(|v| v.parse().ok());
    props.rotation = extract_first_attr(xml, "a:bodyPr", "rot").and_then(|v| v.parse().ok());
    props.raw_xml = Some(xml.to_string());
    props
}
/// Parses a text paragraph.
fn parse_text_paragraph(xml: &str) -> FTextParagraph {
    let mut para = FTextParagraph::default();
    if let Some(ppr) = extract_element_inner(xml, "a:pPr") {
        if let Some(algn) = extract_first_attr(&ppr, "a:pPr", "algn") {
            para.alignment = Some(match algn.as_str() {
                "ctr" => FParagraphAlignment::Center,
                "r" => FParagraphAlignment::Right,
                "just" => FParagraphAlignment::Justify,
                "justLow" => FParagraphAlignment::JustifyLow,
                "dist" => FParagraphAlignment::Distributed,
                "thaiDist" => FParagraphAlignment::ThaiDistributed,
                _ => FParagraphAlignment::Left,
            });
        }
        para.indent_level = extract_first_attr(&ppr, "a:pPr", "lvl").and_then(|v| v.parse().ok());
        para.line_spacing = extract_element_inner(&ppr, "a:lnSpc")
            .and_then(|spc| extract_first_attr(&spc, "a:spcPct", "val"))
            .and_then(|v| v.parse().ok());
        para.space_before = extract_element_inner(&ppr, "a:spcBef")
            .and_then(|spc| extract_first_attr(&spc, "a:spcPts", "val"))
            .and_then(|v| v.parse().ok());
        para.space_after = extract_element_inner(&ppr, "a:spcAft")
            .and_then(|spc| extract_first_attr(&spc, "a:spcPts", "val"))
            .and_then(|v| v.parse().ok());
        if let Some(bu_char) = extract_element_inner(&ppr, "a:buChar") {
            para.bullet_char = extract_first_attr(&bu_char, "a:buChar", "char");
        }
    }
    for r_xml in collect_elements(xml, "a:r") {
        para.runs.push(parse_text_run(&r_xml));
    }
    // Also handle line breaks within paragraph.
    for br_xml in collect_elements(xml, "a:br") {
        let mut run = FRun::default();
        run.text = "\n".to_string();
        para.runs.push(run);
    }
    para.raw_xml = Some(xml.to_string());
    para
}
/// Parses a text run.
fn parse_text_run(xml: &str) -> FRun {
    let mut run = FRun::default();
    if let Some(rpr) = extract_element_inner(xml, "a:rPr") {
        run.properties = parse_run_properties(&rpr);
    }
    if let Some(t) = extract_element_inner(xml, "a:t") {
        run.text = decode_xml_text(&t);
    }
    run.raw_xml = Some(xml.to_string());
    run
}
/// Parses run properties.
fn parse_run_properties(xml: &str) -> FRunProperties {
    let mut props = FRunProperties::default();
    props.bold = flag_present(xml, "a:b");
    props.italic = flag_present(xml, "a:i");
    props.underline = flag_present(xml, "a:u");
    props.strike = flag_present(xml, "a:strike");
    props.caps = flag_present(xml, "a:caps");
    props.small_caps = flag_present(xml, "a:smallCaps");
    if let Some(sz) = extract_first_attr(xml, "a:rPr", "sz") {
        props.size = sz.parse().ok();
    }
    if let Some(font) = extract_first_attr(xml, "a:latin", "typeface") {
        props.font_family = Some(font);
    }
    if let Some(font_ea) = extract_first_attr(xml, "a:ea", "typeface") {
        props.font_family_east_asia = Some(font_ea);
    }
    if let Some(color) = extract_element_inner(xml, "a:solidFill")
        .and_then(|fill| extract_first_attr(&fill, "a:srgbClr", "val"))
    {
        props.color = Some(color);
    }
    if let Some(highlight) = extract_first_attr(xml, "a:highlight", "val") {
        props.highlight = Some(highlight);
    }
    if let Some(baseline) = extract_first_attr(xml, "a:rPr", "baseline") {
        props.baseline = baseline.parse().ok();
    }
    if let Some(spacing) = extract_first_attr(xml, "a:rPr", "spc") {
        props.spacing = spacing.parse().ok();
    }
    props.raw_xml = Some(xml.to_string());
    props
}
/// Parses a table.
fn parse_table(xml: &str) -> FTable {
    let mut table = FTable::default();
    if let Some(tbl_pr) = extract_element_inner(xml, "a:tblPr") {
        table.style_id = extract_first_attr(&tbl_pr, "a:tblPr", "styleId");
        table.first_row =
            extract_first_attr(&tbl_pr, "a:tblPr", "firstRow").map(|v| v == "1" || v == "true");
        table.banded_rows =
            extract_first_attr(&tbl_pr, "a:tblPr", "bandRow").map(|v| v == "1" || v == "true");
    }
    if let Some(tbl_grid) = extract_element_inner(xml, "a:tblGrid") {
        for grid_col in collect_elements(&tbl_grid, "a:gridCol") {
            if let Some(w) = extract_first_attr(&grid_col, "a:gridCol", "w") {
                if let Ok(width) = w.parse::<u32>() {
                    table.column_widths.push(width);
                }
            }
        }
    }
    for tr_xml in collect_elements(xml, "a:tr") {
        let mut row = FTableRow::default();
        row.height = extract_first_attr(&tr_xml, "a:tr", "h").and_then(|v| v.parse().ok());
        for tc_xml in collect_elements(&tr_xml, "a:tc") {
            let mut cell = FTableCell::default();
            if let Some(tc_pr) = extract_element_inner(&tc_xml, "a:tcPr") {
                cell.fill_color = extract_element_inner(&tc_pr, "a:solidFill")
                    .and_then(|fill| extract_first_attr(&fill, "a:srgbClr", "val"));
                cell.vertical_anchor = extract_first_attr(&tc_pr, "a:tcPr", "anchor");
                cell.row_span = extract_first_attr(&tc_pr, "a:tcPr", "rowSpan")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                cell.column_span = extract_first_attr(&tc_pr, "a:tcPr", "gridSpan")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
            }
            for p_xml in collect_elements(&tc_xml, "a:p") {
                cell.paragraphs.push(parse_text_paragraph(&p_xml));
            }
            cell.raw_xml = Some(tc_xml.clone());
            row.cells.push(cell);
        }
        row.raw_xml = Some(tr_xml.clone());
        table.rows.push(row);
    }
    table.raw_xml = Some(xml.to_string());
    table
}
/// Parses theme from XML bytes.
fn parse_theme(xml: &[u8]) -> crate::FPptResult<FTheme> {
    let text = std::str::from_utf8(xml)
        .map_err(|e| FPptError::Xml(format!("theme is not valid utf-8: {}", e)))?;
    let mut theme = FTheme::default();
    theme.name = extract_first_attr(text, "a:theme", "name");
    if let Some(clr_scheme) = extract_element_inner(text, "a:clrScheme") {
        theme.color_scheme = parse_color_scheme(&clr_scheme);
    }
    if let Some(font_scheme) = extract_element_inner(text, "a:fontScheme") {
        theme.font_scheme = parse_font_scheme(&font_scheme);
    }
    theme.raw_xml = Some(text.to_string());
    Ok(theme)
}
/// Parses color scheme.
fn parse_color_scheme(xml: &str) -> FColorScheme {
    let mut scheme = FColorScheme::default();
    scheme.name = extract_first_attr(xml, "a:clrScheme", "name");
    scheme.dark1 = extract_element_inner(xml, "a:dk1")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.light1 = extract_element_inner(xml, "a:lt1")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.dark2 = extract_element_inner(xml, "a:dk2")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.light2 = extract_element_inner(xml, "a:lt2")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.accent1 = extract_element_inner(xml, "a:accent1")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.accent2 = extract_element_inner(xml, "a:accent2")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.accent3 = extract_element_inner(xml, "a:accent3")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.accent4 = extract_element_inner(xml, "a:accent4")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.accent5 = extract_element_inner(xml, "a:accent5")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.accent6 = extract_element_inner(xml, "a:accent6")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.hyperlink = extract_element_inner(xml, "a:hlink")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme.followed_hyperlink = extract_element_inner(xml, "a:folHlink")
        .and_then(|c| extract_first_attr(&c, "a:srgbClr", "val"));
    scheme
}
/// Parses font scheme.
fn parse_font_scheme(xml: &str) -> FFontScheme {
    let mut scheme = FFontScheme::default();
    scheme.name = extract_first_attr(xml, "a:fontScheme", "name");
    if let Some(major) = extract_element_inner(xml, "a:majorFont") {
        scheme.major_latin = extract_first_attr(&major, "a:latin", "typeface");
        scheme.major_east_asia = extract_first_attr(&major, "a:ea", "typeface");
    }
    if let Some(minor) = extract_element_inner(xml, "a:minorFont") {
        scheme.minor_latin = extract_first_attr(&minor, "a:latin", "typeface");
        scheme.minor_east_asia = extract_first_attr(&minor, "a:ea", "typeface");
    }
    scheme
}
