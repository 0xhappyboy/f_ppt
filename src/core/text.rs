//! Text body structures for shapes.
/// Paragraph alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FParagraphAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
    JustifyLow,
    Distributed,
    ThaiDistributed,
}
/// Run-level formatting properties.
#[derive(Debug, Clone, Default)]
pub struct FRunProperties {
    /// Bold flag; `None` means unspecified (inherited).
    pub bold: Option<bool>,
    /// Italic flag; `None` means unspecified.
    pub italic: Option<bool>,
    /// Underline flag; `None` means unspecified.
    pub underline: Option<bool>,
    /// Strikethrough flag; `None` means unspecified.
    pub strike: Option<bool>,
    /// Font size in hundredths of a point.
    pub size: Option<u32>,
    /// Font family name.
    pub font_family: Option<String>,
    /// East Asian font family name.
    pub font_family_east_asia: Option<String>,
    /// Foreground color as hex string without `#`.
    pub color: Option<String>,
    /// Highlight color name.
    pub highlight: Option<String>,
    /// All caps flag.
    pub caps: Option<bool>,
    /// Small caps flag.
    pub small_caps: Option<bool>,
    /// Baseline offset percentage (superscript/subscript).
    pub baseline: Option<i32>,
    /// Character spacing in points.
    pub spacing: Option<i32>,
    /// Raw `<a:rPr>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// A run of text with uniform formatting.
#[derive(Debug, Clone, Default)]
pub struct FRun {
    /// Text content of the run.
    pub text: String,
    /// Formatting applied to the run.
    pub properties: FRunProperties,
    /// Raw `<a:r>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// A paragraph of text within a text body.
#[derive(Debug, Clone, Default)]
pub struct FTextParagraph {
    /// Alignment of the paragraph.
    pub alignment: Option<FParagraphAlignment>,
    /// Ordered list of runs.
    pub runs: Vec<FRun>,
    /// Bullet character (None = no bullet).
    pub bullet_char: Option<String>,
    /// Indent level (0-based).
    pub indent_level: Option<u32>,
    /// Line spacing percentage.
    pub line_spacing: Option<u32>,
    /// Space before paragraph in points.
    pub space_before: Option<u32>,
    /// Space after paragraph in points.
    pub space_after: Option<u32>,
    /// Raw `<a:p>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// Properties controlling text body layout.
#[derive(Debug, Clone, Default)]
pub struct FTextBodyProperties {
    /// Wrap text within shape bounds.
    pub wrap: Option<bool>,
    /// Anchor position (t, ctr, b).
    pub anchor: Option<String>,
    /// Left inset in EMUs.
    pub inset_left: Option<u32>,
    /// Top inset in EMUs.
    pub inset_top: Option<u32>,
    /// Right inset in EMUs.
    pub inset_right: Option<u32>,
    /// Bottom inset in EMUs.
    pub inset_bottom: Option<u32>,
    /// Number of columns.
    pub num_columns: Option<u32>,
    /// Rotation in degrees.
    pub rotation: Option<i32>,
    /// Raw `<a:bodyPr>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
