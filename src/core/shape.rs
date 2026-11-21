//! Shape structures for slides.
use super::text::{FTextBodyProperties, FTextParagraph};
/// Types of shapes in a slide.
#[derive(Debug, Clone)]
pub enum FShape {
    /// A text box shape.
    TextBox(FShapeContainer),
    /// An auto shape (rectangle, ellipse, etc.).
    AutoShape(FAutoShape),
    /// A picture.
    Picture(FPicture),
    /// A group of shapes.
    Group(FGroupShape),
    /// A connector (line).
    Connector(FConnector),
    /// A graphic frame (table, chart, etc.).
    GraphicFrame(FGraphicFrame),
    /// Placeholder shape.
    Placeholder(FPlaceholder),
}
/// Container for shapes with text content.
#[derive(Debug, Clone, Default)]
pub struct FShapeContainer {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Position and size.
    pub properties: FShapeProperties,
    /// Text body content.
    pub text_body: FTextBody,
}
/// Auto shape with preset geometry.
#[derive(Debug, Clone, Default)]
pub struct FAutoShape {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Position and size.
    pub properties: FShapeProperties,
    /// Text body content.
    pub text_body: FTextBody,
    /// Preset geometry type.
    pub shape_type: FAutoShapeType,
    /// Fill color (solid).
    pub fill_color: Option<String>,
    /// Line color.
    pub line_color: Option<String>,
    /// Line width in EMUs.
    pub line_width: Option<u32>,
}
/// Preset geometry types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FAutoShapeType {
    #[default]
    Rectangle,
    Ellipse,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    Octagon,
    Star5,
    Star6,
    Arrow,
    Line,
    RoundRectangle,
    SnipRectangle,
    Parallelogram,
    Trapezoid,
    Heart,
    LightningBolt,
    Sun,
    Moon,
    Cloud,
    Arc,
    Pie,
    Donut,
    NoSymbol,
    BlockArc,
    FoldedCorner,
    SmileyFace,
    Teardrop,
    Trapezoid2,
    IrregularSeal1,
    IrregularSeal2,
    LeftRightArrow,
    UpDownArrow,
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,
    BendedArrow,
    UTurnArrow,
    LeftUpArrow,
    BentUpArrow,
    CurvedRightArrow,
    CurvedLeftArrow,
    CurvedUpArrow,
    CurvedDownArrow,
    StripedRightArrow,
    NotchedRightArrow,
    PentagonArrow,
    Chevron,
    RightArrowCallout,
    DownArrowCallout,
    LeftArrowCallout,
    UpArrowCallout,
    LeftRightArrowCallout,
    UpDownArrowCallout,
    QuadArrowCallout,
    CircularArrow,
    FlowChartProcess,
    FlowChartDecision,
    FlowChartTerminator,
    FlowChartConnector,
    FlowChartManualInput,
    FlowChartManualOperation,
    FlowChartPreparation,
    FlowChartData,
    FlowChartInternalStorage,
    FlowChartDocument,
    FlowChartMultidocument,
    FlowChartPredefinedProcess,
    FlowChartStoredData,
    FlowChartSequentialAccessStorage,
    FlowChartMagneticDisk,
    FlowChartDirectAccessStorage,
    FlowChartDisplay,
    MathPlus,
    MathMinus,
    MathMultiply,
    MathDivide,
    MathEqual,
    MathNotEqual,
    Gear6,
    Gear9,
    Funnel,
    ChartPlus,
    ChartStar,
    ChartX,
}
/// A picture shape.
#[derive(Debug, Clone, Default)]
pub struct FPicture {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Position and size.
    pub properties: FShapeProperties,
    /// Relationship ID of the image.
    pub relationship_id: String,
    /// Image file extension hint.
    pub content_type: Option<String>,
    /// Alternative text description.
    pub alt_text: Option<String>,
    /// Original relationship target path of the image part.
    pub target: Option<String>,
}
/// A group of shapes.
#[derive(Debug, Clone, Default)]
pub struct FGroupShape {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Child shapes.
    pub shapes: Vec<FShape>,
    /// Group transform.
    pub properties: FShapeProperties,
}
/// A connector (line) shape.
#[derive(Debug, Clone, Default)]
pub struct FConnector {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Position and size.
    pub properties: FShapeProperties,
    /// Connector type (straight, bent, curved).
    pub connector_type: Option<String>,
    /// Line color.
    pub line_color: Option<String>,
    /// Line width in EMUs.
    pub line_width: Option<u32>,
    /// Arrow at start.
    pub arrow_start: Option<String>,
    /// Arrow at end.
    pub arrow_end: Option<String>,
}
/// A graphic frame (table, chart).
#[derive(Debug, Clone, Default)]
pub struct FGraphicFrame {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Position and size.
    pub properties: FShapeProperties,
    /// Table content if this frame holds a table.
    pub table: Option<super::table::FTable>,
    /// Chart relationship ID if this frame holds a chart.
    pub chart_relationship_id: Option<String>,
}
/// Placeholder types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FPlaceholderType {
    #[default]
    None,
    Title,
    Body,
    CenterTitle,
    SubTitle,
    Date,
    Footer,
    Header,
    SlideNumber,
    Picture,
    Chart,
    Table,
    Media,
    Object,
}
/// A placeholder shape.
#[derive(Debug, Clone, Default)]
pub struct FPlaceholder {
    /// Shape identifier.
    pub id: u32,
    /// Shape name.
    pub name: String,
    /// Position and size.
    pub properties: FShapeProperties,
    /// Placeholder type.
    pub placeholder_type: FPlaceholderType,
    /// Placeholder index.
    pub index: Option<u32>,
    /// Text body content.
    pub text_body: FTextBody,
}
/// Position and size of a shape.
#[derive(Debug, Clone, Copy, Default)]
pub struct FShapeProperties {
    /// Left position in EMUs.
    pub x: i64,
    /// Top position in EMUs.
    pub y: i64,
    /// Width in EMUs.
    pub width: u32,
    /// Height in EMUs.
    pub height: u32,
    /// Rotation in degrees (clockwise).
    pub rotation: Option<i32>,
    /// Horizontal flip.
    pub flip_horizontal: bool,
    /// Vertical flip.
    pub flip_vertical: bool,
}
/// Text body container.
#[derive(Debug, Clone, Default)]
pub struct FTextBody {
    /// Body properties.
    pub body_properties: FTextBodyProperties,
    /// Paragraphs.
    pub paragraphs: Vec<FTextParagraph>,
}
