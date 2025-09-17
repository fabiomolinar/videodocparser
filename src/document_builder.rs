//! Document Builder Module
//!
//! Handles the creation of the final output document, such as a searchable PDF.

use crate::ocr::{OcrFrameResult};
use anyhow::{Context, Result};
use image::{ImageBuffer, ImageOutputFormat, Rgb};
use log::info;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

// Standard PDF page sizes in points (1/72 inch).
const A4_WIDTH_PT: f32 = 595.0;
const A4_HEIGHT_PT: f32 = 842.0;

/// Builds a searchable PDF from frames and their corresponding OCR results.
pub fn build_pdf(
    frames: &[ImageBuffer<Rgb<u8>, Vec<u8>>],
    ocr_results: &[OcrFrameResult],
    output_path: &Path, // Changed from output_dir to the full file path
) -> Result<()> {
    let mut pdf = Pdf::new();
    let mut ref_counter = std::iter::successors(Some(1), |n| Some(n + 1));

    // Define top-level document objects
    let catalog_ref = Ref::new(ref_counter.next().unwrap());
    let page_tree_ref = Ref::new(ref_counter.next().unwrap());
    let font_ref = Ref::new(ref_counter.next().unwrap());
    pdf.catalog(catalog_ref).pages(page_tree_ref);
    pdf.type1_font(font_ref).base_font(Name(b"Helvetica"));

    let ocr_map: HashMap<usize, &OcrFrameResult> =
        ocr_results.iter().map(|r| (r.frame_index, r)).collect();

    // Pre-allocate all page Refs
    let page_refs: Vec<Ref> = (0..frames.len())
        .map(|_| Ref::new(ref_counter.next().unwrap()))
        .collect();

    // The main loop is now much cleaner. It calls a helper to build each page.
    for (i, frame) in frames.iter().enumerate() {
        build_single_page(
            &mut pdf,
            &mut ref_counter,
            page_refs[i],
            page_tree_ref,
            font_ref,
            frame,
            ocr_map.get(&i),
        )?;
    }

    // Write the page tree
    pdf.pages(page_tree_ref).kids(page_refs).count(frames.len() as i32);

    // Ensure parent directory exists and write the file
    if let Some(parent_dir) = output_path.parent() {
        fs::create_dir_all(parent_dir).context("Failed to create PDF parent directory")?;
    }
    info!("Writing PDF to {:?}", output_path);
    fs::write(output_path, pdf.finish())
        .context("Failed to write PDF file")?;

    Ok(())
}

/// Helper function that constructs all the objects for a single page.
#[allow(clippy::too_many_arguments)]
fn build_single_page(
    pdf: &mut Pdf,
    ref_counter: &mut dyn Iterator<Item = i32>,
    page_ref: Ref,
    page_tree_ref: Ref,
    font_ref: Ref,
    frame: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    ocr_result: Option<&&OcrFrameResult>,
) -> Result<()> {
    let content_ref = Ref::new(ref_counter.next().unwrap());
    let image_ref = Ref::new(ref_counter.next().unwrap());
    let image_name_str = format!("Frame{}", image_ref.get());
    let image_name = Name(image_name_str.as_bytes());

    // 1. Determine page orientation and scaling
    let (image_width, image_height) = frame.dimensions();
    let (page_width, page_height) = if image_width > image_height {
        (A4_HEIGHT_PT, A4_WIDTH_PT) // Landscape
    } else {
        (A4_WIDTH_PT, A4_HEIGHT_PT) // Portrait
    };
    let scale_x = page_width / image_width as f32;
    let scale_y = page_height / image_height as f32;
    let scale_factor = scale_x.min(scale_y);
    let scaled_width = image_width as f32 * scale_factor;
    let scaled_height = image_height as f32 * scale_factor;
    let offset_x = (page_width - scaled_width) / 2.0;
    let offset_y = (page_height - scaled_height) / 2.0;

    // 2. Write the page object dictionary
    let mut page = pdf.page(page_ref);
    page.media_box(Rect::new(0.0, 0.0, page_width, page_height));
    page.parent(page_tree_ref);
    page.contents(content_ref);
    let mut resources = page.resources();
    resources.fonts().pair(Name(b"Helvetica"), font_ref);
    resources.x_objects().pair(image_name, image_ref);
    resources.finish();
    page.finish();

    // 3. Prepare the image and content streams
    let mut content = Content::new();
    content.save_state();
    content.transform([scaled_width, 0.0, 0.0, scaled_height, offset_x, offset_y]);
    content.x_object(image_name);
    content.restore_state();

    if let Some(ocr) = ocr_result {
        // Create a temporary font object just for calculating text widths.
        content.begin_text();
        content.set_text_rendering_mode(pdf_writer::types::TextRenderingMode::Invisible);
        for word in &ocr.words {
            if word.confidence < 50.0 { continue; }

            let (x1, y1, _x2, y2) = word.bbox;
            let scaled_font_size = (y2 - y1) as f32 * scale_factor;
            let scaled_bbox_x1 = x1 as f32 * scale_factor + offset_x;
            let original_flipped_y = image_height as i32 - y2;
            let scaled_pdf_y = original_flipped_y as f32 * scale_factor + offset_y;

            content.set_font(Name(b"Helvetica"), scaled_font_size);
            content.set_text_matrix([1.0, 0.0, 0.0, 1.0, scaled_bbox_x1, scaled_pdf_y]);

            content.show(Str(word.text.as_bytes()));
        }
        content.end_text();
    }
    pdf.stream(content_ref, &content.finish());

    // 4. Write the image XObject with JPEG compression
    let mut encoded_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut encoded_bytes);
    frame.write_to(&mut cursor, ImageOutputFormat::Jpeg(85))?;
    
    let mut image_xobject = pdf.image_xobject(image_ref, &encoded_bytes);
    image_xobject.filter(Filter::DctDecode);
    image_xobject.width(image_width as i32);
    image_xobject.height(image_height as i32);
    image_xobject.color_space().device_rgb();
    image_xobject.bits_per_component(8);
    image_xobject.finish();

    Ok(())
}