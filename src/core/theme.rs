//! Theme structures.
/// A color scheme.
#[derive(Debug, Clone, Default)]
pub struct FColorScheme {
    /// Name of the color scheme.
    pub name: Option<String>,
    /// Dark 1 color.
    pub dark1: Option<String>,
    /// Light 1 color.
    pub light1: Option<String>,
    /// Dark 2 color.
    pub dark2: Option<String>,
    /// Light 2 color.
    pub light2: Option<String>,
    /// Accent 1 color.
    pub accent1: Option<String>,
    /// Accent 2 color.
    pub accent2: Option<String>,
    /// Accent 3 color.
    pub accent3: Option<String>,
    /// Accent 4 color.
    pub accent4: Option<String>,
    /// Accent 5 color.
    pub accent5: Option<String>,
    /// Accent 6 color.
    pub accent6: Option<String>,
    /// Hyperlink color.
    pub hyperlink: Option<String>,
    /// Followed hyperlink color.
    pub followed_hyperlink: Option<String>,
}
/// A font scheme.
#[derive(Debug, Clone, Default)]
pub struct FFontScheme {
    /// Name of the font scheme.
    pub name: Option<String>,
    /// Major (heading) Latin font.
    pub major_latin: Option<String>,
    /// Major East Asian font.
    pub major_east_asia: Option<String>,
    /// Minor (body) Latin font.
    pub minor_latin: Option<String>,
    /// Minor East Asian font.
    pub minor_east_asia: Option<String>,
}
/// Theme definition.
#[derive(Debug, Clone, Default)]
pub struct FTheme {
    /// Theme name.
    pub name: Option<String>,
    /// Color scheme.
    pub color_scheme: FColorScheme,
    /// Font scheme.
    pub font_scheme: FFontScheme,
    /// Raw theme XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
