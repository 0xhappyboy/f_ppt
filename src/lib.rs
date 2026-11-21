//! PowerPoint document parsing engine.
mod core;
mod error;
mod io;
mod pkg;
mod xml;
pub use core::media::{FImage, FMedia, FMediaType, FVideo};
pub use core::presentation::{FPresentation, FSlideId, FSlideSize};
pub use core::shape::{
    FAutoShape, FAutoShapeType, FConnector, FGraphicFrame, FGroupShape, FPicture, FPlaceholder,
    FPlaceholderType, FShape, FShapeContainer, FShapeProperties, FTextBody,
};
pub use core::slide::{FBackground, FBackgroundType, FSlide, FSlideLayout, FSlideTransition};
pub use core::table::{FTable, FTableCell, FTableCellProperties, FTableRow};
pub use core::text::{
    FParagraphAlignment, FRun, FRunProperties, FTextBodyProperties, FTextParagraph,
};
pub use core::theme::{FColorScheme, FFontScheme, FTheme};
pub use error::{FPptError, FPptResult};
pub use io::*;
pub use pkg::{FPackage, FPackagePart};
pub use xml::*;
