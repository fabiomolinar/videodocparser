//! Video Processing Module
//!
//! Handles the decoding of video files and extraction of individual frames
//! using the ffmpeg-next crate.

use ffmpeg_next as ffmpeg;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{Context as ScalingContext, flag::Flags};
use ffmpeg::util::frame::video::Video;
use image::{ImageBuffer, Rgb};
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use log::info;

/// Attempts to get the total number of frames from video metadata.
///
/// This function is much faster than decoding the whole video, but the
/// result can be an estimate for variable frame rate (VFR) videos.
pub fn get_frame_count(path: &Path) -> Result<u64> {
    ffmpeg::init().context("Failed to initialize FFmpeg")?;
    let ictx = input(path).context("Failed to open input file for frame count")?;
    let stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or_else(|| anyhow!("Could not find video stream in file"))?;

    // First, try the most direct method if available in the container
    let frame_count = stream.frames();
    if frame_count > 0 {
        return Ok(frame_count as u64);
    }

    // Fallback: Calculate from duration and average frame rate
    let duration = ictx.duration();
    let frame_rate = stream.avg_frame_rate();

    if duration > 0 && frame_rate.0 > 0 && frame_rate.1 > 0 {
        // Duration is in AV_TIME_BASE units (microseconds), so convert to seconds
        let duration_secs = duration as f64 / 1_000_000.0;
        let fps = frame_rate.0 as f64 / frame_rate.1 as f64;
        let estimated_frames = (duration_secs * fps).round() as u64;
        return Ok(estimated_frames);
    }

    Err(anyhow!("Could not determine frame count from video metadata"))
}

/// Processes video frames using a streaming approach.
///
/// Instead of returning a Vec of all frames, this function decodes one frame at a time
/// and passes it to the `on_frame` closure provided by the caller. This keeps memory
/// usage low and constant.
pub fn process_frames_stream<F>(path: &Path, mut on_frame: F) -> Result<()>
where
    F: FnMut(ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<()>,
{
    ffmpeg::init().context("Failed to initialize FFmpeg")?;
     
    let mut ictx = input(path).context("Failed to open input file")?;
    let input = ictx
        .streams()
        .best(Type::Video)
        .context("Could not find video stream")?;
    let video_stream_index = input.index();

    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())
        .context("Failed to create decoder context")?;
    let mut decoder = context_decoder.decoder().video()
        .context("Failed to create video decoder")?;

    let mut scaler = ScalingContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    ).context("Failed to create scaler")?;

    let mut frame_count = 0;
    let mut receive_and_process_decoded_frames = 
        |decoder: &mut ffmpeg::decoder::Video| -> Result<()> {
            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = Video::empty();
                scaler.run(&decoded, &mut rgb_frame).context("Scaler failed")?;
                
                let frame_data = rgb_frame.data(0);
                let width = rgb_frame.width() as usize;
                let height = rgb_frame.height() as usize;
                let stride = rgb_frame.stride(0) as usize;

                if stride == 0 {
                    return Err(anyhow::anyhow!("Invalid frame stride"));
                }
                
                let mut new_vec = Vec::with_capacity(width * height * 3);
                for y in 0..height {
                    let start_index = y * stride;
                    let end_index = start_index + (width * 3);
                    if end_index > frame_data.len() {
                        return Err(anyhow::anyhow!("Frame data is smaller than expected"));
                    }
                    new_vec.extend_from_slice(&frame_data[start_index..end_index]);
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> = 
                    ImageBuffer::from_vec(width as u32, height as u32, new_vec)
                        .context("Failed to create image buffer from frame data")?;

                // Pass the processed frame to the callback instead of collecting it.
                on_frame(img)?;
                frame_count += 1;
            }
            Ok(())
        };

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet).context("Failed to send packet to decoder")?;
            receive_and_process_decoded_frames(&mut decoder)?;
        }
    }
    decoder.send_eof()?;
    receive_and_process_decoded_frames(&mut decoder)?;

    info!("Finished processing {} frames from video stream.", frame_count);
    Ok(())
}