//! Root presentation structure.
use super::slide::FSlide;
use super::theme::FTheme;
use crate::FPackage;
/// Identifies a slide within the presentation.
#[derive(Debug, Clone, Default)]
pub struct FSlideId {
    /// Relationship identifier pointing to the slide part.
    pub relationship_id: String,
    /// 1-based slide number in presentation order.
    pub slide_number: u32,
    /// The loaded slide content, if parsed.
    pub slide: Option<Box<FSlide>>,
}
/// Slide dimensions in EMUs (English Metric Units).
#[derive(Debug, Clone)]
pub struct FSlideSize {
    /// Width in EMUs.
    pub width: u32,
    /// Height in EMUs.
    pub height: u32,
    /// Type token (screen4x3, screen16x9, custom).
    pub size_type: Option<String>,
}
impl Default for FSlideSize {
    fn default() -> Self {
        FSlideSize {
            width: 9144000,
            height: 6858000,
            size_type: Some("screen4x3".to_string()),
        }
    }
}
/// Root of the PowerPoint document data model.
#[derive(Debug, Clone, Default)]
pub struct FPresentation {
    /// Package-level parts that are not modeled explicitly.
    pub package: FPackage,
    /// Ordered list of slides.
    pub slides: Vec<FSlideId>,
    /// Slide dimensions.
    pub slide_size: FSlideSize,
    /// Theme definitions.
    pub theme: FTheme,
    /// Whether the document has been modified since it was loaded.
    pub dirty: bool,
}
impl FPresentation {
    /// Creates an empty in-memory presentation.
    pub fn new_empty() -> Self {
        FPresentation::default()
    }
    /// Loads a presentation from a file on disk.
    pub fn with_path<P: AsRef<std::path::Path>>(path: P) -> crate::FPptResult<Self> {
        let data = std::fs::read(path)?;
        Self::with_bytes(&data)
    }
    /// Parses a presentation from its raw `.pptx` bytes.
    pub fn with_bytes(data: &[u8]) -> crate::FPptResult<Self> {
        crate::io::reader::parse_presentation(data)
    }
    /// Serializes the presentation back into `.pptx` bytes.
    pub fn to_bytes(&self) -> crate::FPptResult<Vec<u8>> {
        crate::io::writer::serialize_presentation(self)
    }
    /// Marks the presentation as modified.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    /// Clears the modified flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
    /// Returns the number of slides.
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }
    /// Returns plain text content of all slides.
    pub fn plain_text(&self) -> String {
        let mut text = String::new();
        for slide_id in &self.slides {
            if let Some(slide) = &slide_id.slide {
                for shape in &slide.shapes {
                    collect_shape_text(shape, &mut text);
                }
            }
            text.push('\n');
        }
        text
    }
}
fn collect_shape_text(shape: &super::shape::FShape, out: &mut String) {
    match shape {
        super::shape::FShape::TextBox(tb) => {
            for para in &tb.text_body.paragraphs {
                for run in &para.runs {
                    out.push_str(&run.text);
                }
                out.push('\n');
            }
        }
        super::shape::FShape::AutoShape(as_) => {
            for para in &as_.text_body.paragraphs {
                for run in &para.runs {
                    out.push_str(&run.text);
                }
                out.push('\n');
            }
        }
        super::shape::FShape::Placeholder(ph) => {
            for para in &ph.text_body.paragraphs {
                for run in &para.runs {
                    out.push_str(&run.text);
                }
                out.push('\n');
            }
        }
        super::shape::FShape::Group(g) => {
            for child in &g.shapes {
                collect_shape_text(child, out);
            }
        }
        _ => {}
    }
}
