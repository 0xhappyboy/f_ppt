#[cfg(test)]
mod tests {
    use f_ppt::{
        FAutoShape, FAutoShapeType, FBackground, FBackgroundType, FColorScheme, FConnector,
        FFontScheme, FGraphicFrame, FGroupShape, FImage, FMedia, FMediaType, FPackagePart,
        FParagraphAlignment, FPicture, FPlaceholder, FPlaceholderType, FPresentation, FRun,
        FRunProperties, FShape, FShapeContainer, FShapeProperties, FSlide, FSlideId, FSlideSize,
        FSlideTransition, FTable, FTableCell, FTableRow, FTextBody, FTextBodyProperties,
        FTextParagraph, FTheme,
    };
    use std::io::Read;
    const TEST_IMAGE_URL_1: &str = "https://picsum.photos/id/237/200/200";
    const TEST_IMAGE_URL_2: &str = "https://picsum.photos/id/238/300/200";
    const TEST_IMAGE_URL_3: &str = "https://picsum.photos/id/239/400/300";
    const TEST_IMAGE_URL_4: &str = "https://picsum.photos/id/240/250/250";
    const windows_output_url: &str = "/Users/max/Downloads/tests";
    const macos_output_url: &str = "/Users/max/Downloads/tests";
    /// Output path for demo PPTX file.
    fn demo_path() -> std::path::PathBuf {
        #[cfg(target_os = "windows")]
        {
            std::path::PathBuf::from(windows_output_url).join("demo.pptx")
        }
        #[cfg(target_os = "macos")]
        {
            std::path::PathBuf::from(macos_output_url).join("demo.pptx")
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            std::env::temp_dir().join("demo.pptx")
        }
    }
    /// Downloads an image from URL and returns raw bytes.
    fn download_image(url: &str) -> Vec<u8> {
        ureq::get(url)
            .call()
            .expect("failed to download image")
            .into_body()
            .into_reader()
            .bytes()
            .collect::<Result<Vec<u8>, _>>()
            .expect("failed to read image data")
    }
    /// Detects image format from magic bytes.
    fn detect_image_format(bytes: &[u8]) -> &'static str {
        if bytes.len() >= 8 && bytes[0..4] == [0x89, 0x50, 0x4E, 0x47] {
            "png"
        } else if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
            "jpg"
        } else if bytes.len() >= 6
            && (bytes[0..6] == [0x47, 0x49, 0x46, 0x38, 0x37, 0x61]
                || bytes[0..6] == [0x47, 0x49, 0x46, 0x38, 0x39, 0x61])
        {
            "gif"
        } else if bytes.len() >= 2 && bytes[0..2] == [0x42, 0x4D] {
            "bmp"
        } else {
            "png"
        }
    }
    /// Builds a text run with the given text.
    fn make_run(text: &str) -> FRun {
        FRun {
            text: text.to_string(),
            ..Default::default()
        }
    }
    /// Builds a styled text run.
    fn make_styled_run(text: &str, props: FRunProperties) -> FRun {
        FRun {
            text: text.to_string(),
            properties: props,
            ..Default::default()
        }
    }
    /// Builds a text paragraph with the given text.
    fn make_para(text: &str) -> FTextParagraph {
        FTextParagraph {
            runs: vec![make_run(text)],
            ..Default::default()
        }
    }
    /// Builds a text body with a single paragraph.
    fn make_text_body(text: &str) -> FTextBody {
        FTextBody {
            body_properties: FTextBodyProperties::default(),
            paragraphs: vec![make_para(text)],
        }
    }
    /// Builds a simple text box shape.
    fn make_text_box(id: u32, name: &str, x: i64, y: i64, w: u32, h: u32, text: &str) -> FShape {
        FShape::TextBox(FShapeContainer {
            id,
            name: name.to_string(),
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            text_body: make_text_body(text),
        })
    }
    /// Builds an auto shape with fill and line.
    fn make_auto_shape(
        id: u32,
        name: &str,
        x: i64,
        y: i64,
        w: u32,
        h: u32,
        shape_type: FAutoShapeType,
        fill_color: &str,
        line_color: &str,
        text: &str,
    ) -> FShape {
        FShape::AutoShape(FAutoShape {
            id,
            name: name.to_string(),
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            text_body: make_text_body(text),
            shape_type,
            fill_color: Some(fill_color.to_string()),
            line_color: Some(line_color.to_string()),
            line_width: Some(12700),
        })
    }
    /// Builds a picture shape with target path for media resolution.
    fn make_picture(
        id: u32,
        name: &str,
        x: i64,
        y: i64,
        w: u32,
        h: u32,
        rel_id: &str,
        target_path: &str,
        alt_text: &str,
    ) -> FShape {
        FShape::Picture(FPicture {
            id,
            name: name.to_string(),
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            relationship_id: rel_id.to_string(),
            content_type: None,
            alt_text: Some(alt_text.to_string()),
            target: Some(target_path.to_string()),
        })
    }
    /// Builds a placeholder shape.
    fn make_placeholder(
        id: u32,
        name: &str,
        ph_type: FPlaceholderType,
        x: i64,
        y: i64,
        w: u32,
        h: u32,
        text: &str,
    ) -> FShape {
        FShape::Placeholder(FPlaceholder {
            id,
            name: name.to_string(),
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            placeholder_type: ph_type,
            index: None,
            text_body: make_text_body(text),
        })
    }
    /// Builds a table cell with text.
    fn make_cell(text: &str) -> FTableCell {
        FTableCell {
            paragraphs: vec![make_para(text)],
            ..Default::default()
        }
    }
    /// Builds a styled table cell.
    fn make_styled_cell(text: &str, fill: &str) -> FTableCell {
        FTableCell {
            fill_color: Some(fill.to_string()),
            paragraphs: vec![make_para(text)],
            ..Default::default()
        }
    }
    /// Builds a table shape.
    fn make_table(id: u32, name: &str, x: i64, y: i64, w: u32, h: u32) -> FShape {
        let header_fill = "4472C4";
        let mut row1 = FTableRow::default();
        row1.cells.push(make_styled_cell("Name", header_fill));
        row1.cells.push(make_styled_cell("Value", header_fill));
        row1.cells.push(make_styled_cell("Status", header_fill));
        let mut row2 = FTableRow::default();
        row2.cells.push(make_cell("Item A"));
        row2.cells.push(make_cell("100"));
        row2.cells.push(make_styled_cell("Active", "70AD47"));
        let mut row3 = FTableRow::default();
        row3.cells.push(make_cell("Item B"));
        row3.cells.push(make_cell("200"));
        row3.cells.push(make_styled_cell("Pending", "FFC000"));
        let mut row4 = FTableRow::default();
        row4.cells.push(make_cell("Item C"));
        row4.cells.push(make_cell("300"));
        row4.cells.push(make_styled_cell("Inactive", "FF0000"));
        let table = FTable {
            rows: vec![row1, row2, row3, row4],
            column_widths: vec![2000000, 1500000, 1500000],
            style_id: Some("TableStyle1".to_string()),
            first_row: Some(true),
            banded_rows: Some(true),
            ..Default::default()
        };
        FShape::GraphicFrame(FGraphicFrame {
            id,
            name: name.to_string(),
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            table: Some(table),
            chart_relationship_id: None,
        })
    }
    /// Builds a connector (line) shape.
    fn make_connector(
        id: u32,
        name: &str,
        x: i64,
        y: i64,
        w: u32,
        h: u32,
        line_color: &str,
    ) -> FShape {
        FShape::Connector(FConnector {
            id,
            name: name.to_string(),
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            connector_type: Some("straight".to_string()),
            line_color: Some(line_color.to_string()),
            line_width: Some(19050),
            arrow_start: None,
            arrow_end: Some("triangle".to_string()),
        })
    }
    /// Builds a group shape containing child shapes.
    fn make_group(id: u32, name: &str, x: i64, y: i64, w: u32, h: u32) -> FShape {
        let child1 = make_auto_shape(
            id * 10 + 1,
            "Child1",
            0,
            0,
            500000,
            500000,
            FAutoShapeType::Ellipse,
            "FF6B6B",
            "C92A2A",
            "A",
        );
        let child2 = make_auto_shape(
            id * 10 + 2,
            "Child2",
            600000,
            0,
            500000,
            500000,
            FAutoShapeType::Rectangle,
            "4DABF7",
            "1971C2",
            "B",
        );
        FShape::Group(FGroupShape {
            id,
            name: name.to_string(),
            shapes: vec![child1, child2],
            properties: FShapeProperties {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
        })
    }
    /// Creates the theme for the presentation.
    fn make_theme() -> FTheme {
        FTheme {
            name: Some("Demo Theme".to_string()),
            color_scheme: FColorScheme {
                name: Some("Demo Colors".to_string()),
                dark1: Some("000000".to_string()),
                light1: Some("FFFFFF".to_string()),
                dark2: Some("1F4E79".to_string()),
                light2: Some("E7E6E6".to_string()),
                accent1: Some("4472C4".to_string()),
                accent2: Some("ED7D31".to_string()),
                accent3: Some("A5A5A5".to_string()),
                accent4: Some("FFC000".to_string()),
                accent5: Some("5B9BD5".to_string()),
                accent6: Some("70AD47".to_string()),
                hyperlink: Some("0563C1".to_string()),
                followed_hyperlink: Some("954F72".to_string()),
            },
            font_scheme: FFontScheme {
                name: Some("Demo Fonts".to_string()),
                major_latin: Some("Calibri Light".to_string()),
                major_east_asia: Some("".to_string()),
                minor_latin: Some("Calibri".to_string()),
                minor_east_asia: Some("".to_string()),
            },
            ..Default::default()
        }
    }
    /// Writes a comprehensive PPTX demo covering all features.
    #[test]
    fn write_demo_pptx() {
        let mut pres = FPresentation::new_empty();
        pres.slide_size = FSlideSize {
            width: 9144000,
            height: 6858000,
            size_type: Some("screen4x3".to_string()),
        };
        pres.theme = make_theme();
        // Download 4 test images.
        let img1_bytes = download_image(TEST_IMAGE_URL_1);
        let img2_bytes = download_image(TEST_IMAGE_URL_2);
        let img3_bytes = download_image(TEST_IMAGE_URL_3);
        let img4_bytes = download_image(TEST_IMAGE_URL_4);
        let img1_ext = detect_image_format(&img1_bytes);
        let img2_ext = detect_image_format(&img2_bytes);
        let img3_ext = detect_image_format(&img3_bytes);
        let img4_ext = detect_image_format(&img4_bytes);
        println!("[image1] {} bytes, format: {}", img1_bytes.len(), img1_ext);
        println!("[image2] {} bytes, format: {}", img2_bytes.len(), img2_ext);
        println!("[image3] {} bytes, format: {}", img3_bytes.len(), img3_ext);
        println!("[image4] {} bytes, format: {}", img4_bytes.len(), img4_ext);
        // Store images in package parts.
        let img1_path = format!("ppt/media/image1.{}", img1_ext);
        let img2_path = format!("ppt/media/image2.{}", img2_ext);
        let img3_path = format!("ppt/media/image3.{}", img3_ext);
        let img4_path = format!("ppt/media/image4.{}", img4_ext);
        pres.package.parts.push(FPackagePart {
            path: img1_path.clone(),
            data: img1_bytes,
        });
        pres.package.parts.push(FPackagePart {
            path: img2_path.clone(),
            data: img2_bytes,
        });
        pres.package.parts.push(FPackagePart {
            path: img3_path.clone(),
            data: img3_bytes,
        });
        pres.package.parts.push(FPackagePart {
            path: img4_path.clone(),
            data: img4_bytes,
        });
        // Slide 1: Title slide with background and placeholders.
        let mut slide1 = FSlide::default();
        slide1.background = Some(FBackground {
            fill_type: Some(FBackgroundType::Solid),
            color: Some("1F4E79".to_string()),
            ..Default::default()
        });
        slide1.shapes.push(make_placeholder(
            1,
            "Title Placeholder",
            FPlaceholderType::CenterTitle,
            1000000,
            2000000,
            7000000,
            1000000,
            "Demo PPTX Presentation",
        ));
        slide1.shapes.push(make_placeholder(
            2,
            "Subtitle Placeholder",
            FPlaceholderType::SubTitle,
            1500000,
            3200000,
            6000000,
            500000,
            "Generated by f_ppt engine",
        ));
        slide1.transition = Some(FSlideTransition {
            transition_type: Some("fade".to_string()),
            duration_ms: Some(500),
            advance_on_click: Some(true),
            advance_after_ms: None,
            ..Default::default()
        });
        // Slide 2: Text content with formatting.
        let mut slide2 = FSlide::default();
        slide2.background = Some(FBackground {
            fill_type: Some(FBackgroundType::Solid),
            color: Some("FFFFFF".to_string()),
            ..Default::default()
        });
        let mut title_para = make_para("Text Formatting Examples");
        title_para.alignment = Some(FParagraphAlignment::Center);
        let mut title_body = FTextBody {
            body_properties: FTextBodyProperties::default(),
            paragraphs: vec![title_para],
        };
        title_body.body_properties.anchor = Some("ctr".to_string());
        slide2.shapes.push(FShape::TextBox(FShapeContainer {
            id: 1,
            name: "Title".to_string(),
            properties: FShapeProperties {
                x: 1000000,
                y: 300000,
                width: 7000000,
                height: 600000,
                ..Default::default()
            },
            text_body: title_body,
        }));
        // Text box with mixed formatting.
        let mut mixed_para = FTextParagraph::default();
        mixed_para.runs.push(make_styled_run(
            "Bold ",
            FRunProperties {
                bold: Some(true),
                size: Some(2400),
                ..Default::default()
            },
        ));
        mixed_para.runs.push(make_styled_run(
            "Italic ",
            FRunProperties {
                italic: Some(true),
                size: Some(2400),
                ..Default::default()
            },
        ));
        mixed_para.runs.push(make_styled_run(
            "Underline ",
            FRunProperties {
                underline: Some(true),
                size: Some(2400),
                ..Default::default()
            },
        ));
        mixed_para.runs.push(make_styled_run(
            "Red",
            FRunProperties {
                color: Some("FF0000".to_string()),
                size: Some(2400),
                ..Default::default()
            },
        ));
        mixed_para.runs.push(make_styled_run(
            " and ",
            FRunProperties {
                size: Some(2400),
                ..Default::default()
            },
        ));
        mixed_para.runs.push(make_styled_run(
            "Blue",
            FRunProperties {
                color: Some("0000FF".to_string()),
                size: Some(2400),
                ..Default::default()
            },
        ));
        slide2.shapes.push(FShape::TextBox(FShapeContainer {
            id: 2,
            name: "MixedText".to_string(),
            properties: FShapeProperties {
                x: 800000,
                y: 1200000,
                width: 7500000,
                height: 800000,
                ..Default::default()
            },
            text_body: FTextBody {
                body_properties: FTextBodyProperties::default(),
                paragraphs: vec![mixed_para],
            },
        }));
        // Bullet points.
        let mut bullets = Vec::new();
        for (i, text) in [
            "First bullet point",
            "Second bullet point",
            "Third bullet point",
        ]
        .iter()
        .enumerate()
        {
            let mut para = make_para(text);
            para.bullet_char = Some("•".to_string());
            para.indent_level = Some(0);
            para.space_before = Some(i as u32 * 100);
            bullets.push(para);
        }
        slide2.shapes.push(FShape::TextBox(FShapeContainer {
            id: 3,
            name: "Bullets".to_string(),
            properties: FShapeProperties {
                x: 800000,
                y: 2200000,
                width: 4000000,
                height: 1500000,
                ..Default::default()
            },
            text_body: FTextBody {
                body_properties: FTextBodyProperties::default(),
                paragraphs: bullets,
            },
        }));
        // Auto shapes showcase.
        slide2.shapes.push(make_auto_shape(
            4,
            "Rect1",
            5500000,
            2200000,
            1200000,
            800000,
            FAutoShapeType::Rectangle,
            "4472C4",
            "2F5597",
            "Rectangle",
        ));
        slide2.shapes.push(make_auto_shape(
            5,
            "Ellipse1",
            6900000,
            2200000,
            1200000,
            800000,
            FAutoShapeType::Ellipse,
            "ED7D31",
            "C55A11",
            "Ellipse",
        ));
        slide2.shapes.push(make_auto_shape(
            6,
            "Diamond1",
            5500000,
            3200000,
            1200000,
            800000,
            FAutoShapeType::Diamond,
            "70AD47",
            "548235",
            "Diamond",
        ));
        slide2.shapes.push(make_auto_shape(
            7,
            "Triangle1",
            6900000,
            3200000,
            1200000,
            800000,
            FAutoShapeType::Triangle,
            "FFC000",
            "BF8F00",
            "Triangle",
        ));
        // Connector line.
        slide2.shapes.push(make_connector(
            8, "Line1", 800000, 4000000, 4000000, 0, "4472C4",
        ));
        // Slide 3: Images.
        let mut slide3 = FSlide::default();
        slide3.background = Some(FBackground {
            fill_type: Some(FBackgroundType::Solid),
            color: Some("F5F5F5".to_string()),
            ..Default::default()
        });
        slide3.shapes.push(make_text_box(
            1,
            "ImageTitle",
            1000000,
            200000,
            7000000,
            500000,
            "Image Gallery (4 Network Images)",
        ));
        // Picture 1 with target path.
        slide3.shapes.push(make_picture(
            2,
            "Image1",
            500000,
            1000000,
            2000000,
            2000000,
            "rIdImg1",
            &img1_path,
            "Downloaded image 1",
        ));
        // Picture 2 with target path.
        slide3.shapes.push(make_picture(
            3,
            "Image2",
            2800000,
            1000000,
            3000000,
            2000000,
            "rIdImg2",
            &img2_path,
            "Downloaded image 2",
        ));
        // Picture 3 with target path.
        slide3.shapes.push(make_picture(
            4,
            "Image3",
            6000000,
            1000000,
            2666666,
            2000000,
            "rIdImg3",
            &img3_path,
            "Downloaded image 3",
        ));
        // Picture 4 with target path.
        slide3.shapes.push(make_picture(
            5,
            "Image4",
            2000000,
            3500000,
            2500000,
            2500000,
            "rIdImg4",
            &img4_path,
            "Downloaded image 4",
        ));
        // Group shape with labels.
        slide3.shapes.push(make_group(
            6,
            "LabelGroup",
            5000000,
            3500000,
            1100000,
            500000,
        ));
        // Slide 4: Table.
        let mut slide4 = FSlide::default();
        slide4.background = Some(FBackground {
            fill_type: Some(FBackgroundType::Solid),
            color: Some("FFFFFF".to_string()),
            ..Default::default()
        });
        slide4.shapes.push(make_text_box(
            1,
            "TableTitle",
            1000000,
            300000,
            7000000,
            500000,
            "Data Table Example",
        ));
        slide4.shapes.push(make_table(
            2,
            "DataTable",
            1000000,
            1200000,
            5000000,
            2000000,
        ));
        // Add a note below the table.
        slide4.shapes.push(make_text_box(
            3,
            "TableNote",
            1000000,
            3500000,
            7000000,
            400000,
            "This table demonstrates cell styling, fills, and borders.",
        ));
        // Build slides list.
        pres.slides = vec![
            FSlideId {
                relationship_id: "rId1".to_string(),
                slide_number: 1,
                slide: Some(Box::new(slide1)),
            },
            FSlideId {
                relationship_id: "rId2".to_string(),
                slide_number: 2,
                slide: Some(Box::new(slide2)),
            },
            FSlideId {
                relationship_id: "rId3".to_string(),
                slide_number: 3,
                slide: Some(Box::new(slide3)),
            },
            FSlideId {
                relationship_id: "rId4".to_string(),
                slide_number: 4,
                slide: Some(Box::new(slide4)),
            },
        ];
        // Serialize and write.
        let bytes = pres.to_bytes().expect("failed to serialize presentation");
        println!("[writer] serialized presentation: {} bytes", bytes.len());
        std::fs::write(demo_path(), &bytes).expect("failed to write demo.pptx");
        println!("demo.pptx written to {}", demo_path().display());
    }
    /// Reads the demo PPTX and prints its contents.
    #[test]
    fn read_demo_pptx() {
        let pres = FPresentation::with_path(demo_path()).expect("failed to read demo.pptx");
        println!("=== Presentation Info ===");
        println!("Slide Count: {}", pres.slide_count());
        println!(
            "Slide Size: {} x {} EMU ({:?})",
            pres.slide_size.width, pres.slide_size.height, pres.slide_size.size_type
        );
        println!();
        println!("=== Theme ===");
        if let Some(name) = &pres.theme.name {
            println!("Theme Name: {}", name);
        }
        println!("Colors:");
        println!("  Dark1: {:?}", pres.theme.color_scheme.dark1);
        println!("  Light1: {:?}", pres.theme.color_scheme.light1);
        println!("  Accent1: {:?}", pres.theme.color_scheme.accent1);
        println!("  Accent2: {:?}", pres.theme.color_scheme.accent2);
        println!("Fonts:");
        println!("  Major Latin: {:?}", pres.theme.font_scheme.major_latin);
        println!("  Minor Latin: {:?}", pres.theme.font_scheme.minor_latin);
        println!();
        println!("=== Slides ===");
        for (idx, slide_id) in pres.slides.iter().enumerate() {
            println!("Slide {} (rId={}):", idx + 1, slide_id.relationship_id);
            if let Some(slide) = &slide_id.slide {
                if let Some(bg) = &slide.background {
                    println!("  Background: {:?} color={:?}", bg.fill_type, bg.color);
                }
                if let Some(trans) = &slide.transition {
                    println!(
                        "  Transition: {:?} duration={:?}ms",
                        trans.transition_type, trans.duration_ms
                    );
                }
                println!("  Shapes: {}", slide.shapes.len());
                for (i, shape) in slide.shapes.iter().enumerate() {
                    match shape {
                        FShape::TextBox(tb) => {
                            let text: String = tb
                                .text_body
                                .paragraphs
                                .iter()
                                .flat_map(|p| p.runs.iter())
                                .map(|r| r.text.as_str())
                                .collect();
                            println!(
                                "    [{}] TextBox '{}' at ({},{}) {}x{}: '{}'",
                                i,
                                tb.name,
                                tb.properties.x,
                                tb.properties.y,
                                tb.properties.width,
                                tb.properties.height,
                                text.chars().take(40).collect::<String>()
                            );
                        }
                        FShape::AutoShape(as_) => {
                            let text: String = as_
                                .text_body
                                .paragraphs
                                .iter()
                                .flat_map(|p| p.runs.iter())
                                .map(|r| r.text.as_str())
                                .collect();
                            println!(
                                "    [{}] AutoShape '{}' {:?} fill={:?} line={:?}: '{}'",
                                i,
                                as_.name,
                                as_.shape_type,
                                as_.fill_color,
                                as_.line_color,
                                text.chars().take(30).collect::<String>()
                            );
                        }
                        FShape::Picture(pic) => {
                            println!(
                                "    [{}] Picture '{}' rel={} target={:?} alt={:?} at ({},{}) {}x{}",
                                i,
                                pic.name,
                                pic.relationship_id,
                                pic.target,
                                pic.alt_text,
                                pic.properties.x,
                                pic.properties.y,
                                pic.properties.width,
                                pic.properties.height
                            );
                        }
                        FShape::Group(grp) => {
                            println!(
                                "    [{}] Group '{}' with {} children",
                                i,
                                grp.name,
                                grp.shapes.len()
                            );
                        }
                        FShape::Connector(cxn) => {
                            println!(
                                "    [{}] Connector '{}' type={:?} line={:?}",
                                i, cxn.name, cxn.connector_type, cxn.line_color
                            );
                        }
                        FShape::GraphicFrame(gf) => {
                            if let Some(tbl) = &gf.table {
                                println!(
                                    "    [{}] GraphicFrame '{}' Table: {} rows x {} cols",
                                    i,
                                    gf.name,
                                    tbl.rows.len(),
                                    tbl.column_widths.len()
                                );
                                for (ri, row) in tbl.rows.iter().enumerate() {
                                    let cells: Vec<String> = row
                                        .cells
                                        .iter()
                                        .map(|c| {
                                            c.paragraphs
                                                .iter()
                                                .flat_map(|p| p.runs.iter())
                                                .map(|r| r.text.clone())
                                                .collect::<String>()
                                        })
                                        .collect();
                                    println!("      Row {}: {:?}", ri + 1, cells);
                                }
                            }
                        }
                        FShape::Placeholder(ph) => {
                            let text: String = ph
                                .text_body
                                .paragraphs
                                .iter()
                                .flat_map(|p| p.runs.iter())
                                .map(|r| r.text.as_str())
                                .collect();
                            println!(
                                "    [{}] Placeholder '{:?}' '{}': '{}'",
                                i,
                                ph.placeholder_type,
                                ph.name,
                                text.chars().take(40).collect::<String>()
                            );
                        }
                    }
                }
            }
            println!();
        }
        println!("=== Plain Text ===");
        println!("{}", pres.plain_text());
        println!("=== Package Parts ===");
        println!("Total parts: {}", pres.package.parts.len());
        for part in &pres.package.parts {
            println!("  {} ({} bytes)", part.path, part.data.len());
        }
    }
}
