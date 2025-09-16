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
use anyhow::{Context, Result};
use std::path::Path;
use log::info;

/// Extracts all frames from a video file and returns them as a vector of images.
pub fn extract_frames(path: &Path) -> Result<Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>> {
    ffmpeg::init().context("Failed to initialize FFmpeg")?;
     
    let mut frames = Vec::new();
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

                frames.push(img);                
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

    info!("Finished extracting {} frames total.", frames.len());
    Ok(frames)
}