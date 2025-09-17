//! Document Builder Module
//!
//! Handles the creation of the final output document, such as a searchable PDF.

use crate::ocr::OcrFrameResult;
use anyhow::Result;
use image::{ImageBuffer, ImageOutputFormat, Rgb};
use pdf_writer::{Ref, Content, Filter, Finish, Name, Pdf, Rect, Str};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

// Standard PDF page sizes in points (1/72 inch).
const A4_WIDTH_PT: f32 = 595.0;
const A4_HEIGHT_PT: f32 = 842.0;

/// Builds a searchable PDF from frames and their corresponding OCR results.
pub fn build_pdf(
    frames: &[ImageBuffer<Rgb<u8>, Vec<u8>>],
    ocr_results: &[OcrFrameResult],
    output_path: &Path,
) -> Result<()> {    
    // Initialize PDF document
    let mut counter = std::iter::successors(Some(1), |n| Some (n + 1));
    let mut pdf = Pdf::new();

    // Define references
    let catalog_ref = Ref::new(counter.next().unwrap());
    let page_tree_ref = Ref::new(counter.next().unwrap());    
    let font_ref = Ref::new(counter.next().unwrap());
    let font_name = Name(b"Helvetica");
    pdf.catalog(catalog_ref).pages(page_tree_ref);     
    pdf.type1_font(font_ref).base_font(font_name);
    

    // For robust mapping of frames to OCR results, use a HashMap.
    let ocr_map: HashMap<usize, &OcrFrameResult> =
        ocr_results.iter().map(|r| (r.frame_index, r)).collect();

    let mut kids = Vec::new();
    for _ in 0..frames.len() {
        let page_ref = Ref::new(counter.next().unwrap());
        kids.push(page_ref);
    }
    let kids_len = kids.len() as i32;
    pdf.pages(page_tree_ref).kids(kids.clone()).count(kids_len);

    for (i, frame) in frames.iter().enumerate() {
        let mut page_ref = pdf.page(kids[i]);
        let content_ref = Ref::new(counter.next().unwrap());
        let image_ref = Ref::new(counter.next().unwrap());
        let image_name_string = format!("Frame{}", image_ref.get());
        let image_name = Name(image_name_string.as_bytes());
        page_ref.resources().fonts().pair(font_name, font_ref);
                
        // Determine page orientation based on image aspect ratio.
        let (image_width, image_height) = frame.dimensions();
        let (page_width, page_height) = if image_width > image_height {
            // Landscape
            (A4_HEIGHT_PT, A4_WIDTH_PT)
        } else {
            // Portrait
            (A4_WIDTH_PT, A4_HEIGHT_PT)
        };
        let page_rect = Rect::new(0.0, 0.0, page_width, page_height);

        page_ref.media_box(page_rect);
        page_ref.parent(page_tree_ref);
        page_ref.contents(content_ref);
        page_ref.resources().x_objects().pair(image_name, image_ref);
        page_ref.finish();

        // 2. Calculate scaling factor and offsets to fit and center the image.
        let scale_x = page_width / image_width as f32;
        let scale_y = page_height / image_height as f32;
        let scale_factor = scale_x.min(scale_y); // Use min to fit and preserve aspect ratio

        let scaled_width = image_width as f32 * scale_factor;
        let scaled_height = image_height as f32 * scale_factor;
        let offset_x = (page_width - scaled_width) / 2.0;
        let offset_y = (page_height - scaled_height) / 2.0;
        
        // 3. Embed the frame image onto the page with scaling and translation.
        let filter = Filter::DctDecode;
        let mut encoded_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut encoded_bytes);
        if let Err(e) = frame.write_to(&mut cursor, ImageOutputFormat::Jpeg(85)) {
            eprintln!("Failed to encode image {}: {}. Skipping.", i, e);
            continue;
        }
        let mut image_ref = pdf.image_xobject(image_ref, &encoded_bytes);
        image_ref.filter(filter);
        image_ref.width(image_width as i32);
        image_ref.height(image_height as i32);
        image_ref.color_space().device_rgb();
        image_ref.bits_per_component(8);
        image_ref.finish();

        let mut content = Content::new();
        content.save_state();
        content.transform([scaled_width, 0.0, 0.0, scaled_height, offset_x, offset_y]);
        content.x_object(image_name);
        content.restore_state();
        pdf.stream(content_ref, &content.finish());

        // 4. Overlay OCR text on the image. TBD.
        if let Some(ocr_result) = ocr_map.get(&i) {
            let mut content = Content::new();
            content.begin_text();
            content.set_text_rendering_mode(pdf_writer::types::TextRenderingMode::Invisible);

            for word in &ocr_result.words {
                if word.confidence < 50.0 { continue; }

                // Bounding box as a tuple: (x1, y1, x2, y2)
                let (x1, y1, x2, y2) = &word.bbox;

                // Heuristic to estimate font size, now scaled.
                let scaled_font_size = (y2 - y1) as f32 * scale_factor;

                // Transform the word's coordinates to the new scaled and centered system.
                let scaled_bbox_x1 = x1 as f32 * scale_factor + offset_x;
                
                // PDF Y-coordinate needs to be flipped, then scaled and offset.
                let original_flipped_y = image_height as i32 - y2;
                let scaled_pdf_y = original_flipped_y as f32 * scale_factor + offset_y;
                
                content.set_font(font_name, scaled_font_size);
                
                // Position the text using the transformed coordinates.
                let transform = [1.0, 0.0, 0.0, 1.0, scaled_bbox_x1, scaled_pdf_y];
                content.set_text_matrix(transform);
                
                // A simple heuristic to stretch the word to fit its bounding box width
                let word_width = (x2 - x1) as f32 * scale_factor;
                let text_width = font_ref.width(scaled_font_size, word.text.as_bytes());
                
                if text_width > 0.0 {
                    let horizontal_scaling = (word_width / text_width) * 100.0;
                    content.set_horizontal_scaling(horizontal_scaling);
                }
                
                content.show_text(Str(word.text.as_bytes()));
            }
            content.end_text();
        }
    }
    
    // Join output path and set pdf file name
    let output_file = output_path.join("pdf").join("document.pdf");
    std::fs::write(output_file, pdf.finish())?;
    Ok(())
}