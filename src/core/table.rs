//! Table structures.
use super::text::FTextParagraph;
/// A table cell.
#[derive(Debug, Clone, Default)]
pub struct FTableCell {
    /// Row span (vertical merge).
    pub row_span: u32,
    /// Column span (horizontal merge).
    pub column_span: u32,
    /// Cell fill color.
    pub fill_color: Option<String>,
    /// Cell borders.
    pub borders: Option<FCellBorders>,
    /// Cell margins.
    pub margins: Option<FCellMargins>,
    /// Vertical anchor.
    pub vertical_anchor: Option<String>,
    /// Text paragraphs in the cell.
    pub paragraphs: Vec<FTextParagraph>,
    /// Raw `<a:tc>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// Cell borders.
#[derive(Debug, Clone, Default)]
pub struct FCellBorders {
    pub top: Option<FBorder>,
    pub left: Option<FBorder>,
    pub bottom: Option<FBorder>,
    pub right: Option<FBorder>,
}
/// Border definition.
#[derive(Debug, Clone, Default)]
pub struct FBorder {
    /// Border style (single, double, etc.).
    pub style: Option<String>,
    /// Border width in EMUs.
    pub width: Option<u32>,
    /// Border color.
    pub color: Option<String>,
}
/// Cell margins.
#[derive(Debug, Clone, Default)]
pub struct FCellMargins {
    pub left: Option<u32>,
    pub top: Option<u32>,
    pub right: Option<u32>,
    pub bottom: Option<u32>,
}
/// Table cell properties (tcPr).
#[derive(Debug, Clone, Default)]
pub struct FTableCellProperties {
    /// Row span.
    pub row_span: Option<u32>,
    /// Grid span.
    pub grid_span: Option<u32>,
    /// Horizontal merge.
    pub horizontal_merge: Option<bool>,
    /// Vertical merge.
    pub vertical_merge: Option<bool>,
    /// Fill color.
    pub fill: Option<String>,
}
/// A table row.
#[derive(Debug, Clone, Default)]
pub struct FTableRow {
    /// Cells in the row.
    pub cells: Vec<FTableCell>,
    /// Row height in EMUs.
    pub height: Option<u32>,
    /// Raw `<a:tr>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// A table.
#[derive(Debug, Clone, Default)]
pub struct FTable {
    /// Rows of the table.
    pub rows: Vec<FTableRow>,
    /// Column widths in EMUs.
    pub column_widths: Vec<u32>,
    /// Table style ID.
    pub style_id: Option<String>,
    /// First row is header.
    pub first_row: Option<bool>,
    /// Banded rows.
    pub banded_rows: Option<bool>,
    /// Raw `<a:tbl>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
