//! Slide-level structures.
use super::shape::FShape;
/// Background fill type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FBackgroundType {
    /// No background override.
    None,
    /// Solid color fill.
    Solid,
    /// Gradient fill.
    Gradient,
    /// Picture or texture fill.
    Picture,
}
/// Slide background.
#[derive(Debug, Clone, Default)]
pub struct FBackground {
    /// Type of background fill.
    pub fill_type: Option<FBackgroundType>,
    /// Solid fill color (hex without #).
    pub color: Option<String>,
    /// Raw `<p:bg>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// Slide transition effect.
#[derive(Debug, Clone, Default)]
pub struct FSlideTransition {
    /// Transition type (fade, push, wipe, etc.).
    pub transition_type: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: Option<u32>,
    /// Advance on click.
    pub advance_on_click: Option<bool>,
    /// Auto-advance after milliseconds.
    pub advance_after_ms: Option<u32>,
    /// Raw `<p:transition>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
/// Reference to a slide layout.
#[derive(Debug, Clone, Default)]
pub struct FSlideLayout {
    /// Relationship identifier of the layout part.
    pub relationship_id: Option<String>,
    /// Layout name (Title, TitleAndContent, etc.).
    pub name: Option<String>,
}
/// A single slide in the presentation.
#[derive(Debug, Clone, Default)]
pub struct FSlide {
    /// Shapes on this slide, in z-order.
    pub shapes: Vec<FShape>,
    /// Slide background.
    pub background: Option<FBackground>,
    /// Slide transition.
    pub transition: Option<FSlideTransition>,
    /// Slide layout reference.
    pub layout: Option<FSlideLayout>,
    /// Whether the slide is hidden.
    pub hidden: bool,
    /// Raw `<p:sld>` XML preserved for elements not modeled above.
    pub raw_xml: Option<String>,
}
