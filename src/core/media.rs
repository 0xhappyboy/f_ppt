//! Media structures for images and videos.
/// Type of media content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FMediaType {
    /// Unknown media type.
    #[default]
    Unknown,
    /// JPEG image.
    Jpeg,
    /// PNG image.
    Png,
    /// GIF image.
    Gif,
    /// BMP image.
    Bmp,
    /// TIFF image.
    Tiff,
    /// SVG image.
    Svg,
    /// MP4 video.
    Mp4,
}
/// An image in the presentation.
#[derive(Debug, Clone, Default)]
pub struct FImage {
    /// Relationship ID.
    pub relationship_id: String,
    /// Archive path of the image part.
    pub path: String,
    /// Media type.
    pub media_type: FMediaType,
    /// Image width in EMUs (if known).
    pub width: Option<u32>,
    /// Image height in EMUs (if known).
    pub height: Option<u32>,
    /// Alternative text.
    pub alt_text: Option<String>,
}
/// A video in the presentation.
#[derive(Debug, Clone, Default)]
pub struct FVideo {
    /// Relationship ID.
    pub relationship_id: String,
    /// Archive path of the video part.
    pub path: String,
    /// Media type.
    pub media_type: FMediaType,
    /// Poster image relationship ID.
    pub poster_relationship_id: Option<String>,
}
/// Media collection for the presentation.
#[derive(Debug, Clone, Default)]
pub struct FMedia {
    /// Images by relationship ID.
    pub images: Vec<FImage>,
    /// Videos by relationship ID.
    pub videos: Vec<FVideo>,
}
impl FMedia {
    /// Adds an image to the collection.
    pub fn add_image(&mut self, image: FImage) {
        self.images.push(image);
    }
    /// Adds a video to the collection.
    pub fn add_video(&mut self, video: FVideo) {
        self.videos.push(video);
    }
    /// Finds an image by relationship ID.
    pub fn find_image(&self, relationship_id: &str) -> Option<&FImage> {
        self.images
            .iter()
            .find(|i| i.relationship_id == relationship_id)
    }
    /// Finds a video by relationship ID.
    pub fn find_video(&self, relationship_id: &str) -> Option<&FVideo> {
        self.videos
            .iter()
            .find(|v| v.relationship_id == relationship_id)
    }
}
