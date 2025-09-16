# videodocparser

VideoDocParser is a command-line tool that converts video recordings of documents (e.g., PDFs, Word files, or printed text) into searchable digital formats.

**Still in development.**

## External dependencies

- ffmpeg related: [notes on building](https://github.com/zmwangx/rust-ffmpeg/wiki/Notes-on-building)
- tesseract related: `cmake` [download](https://cmake.org/download/)

## Notes

- To get `tesseract-rs` working, first I had to do a `cargo build --release` to have its binaries created.

## Example

After cloning the repo, one can run the following command to run the process in debug mode (considering the input video is stored at `input/recording.mp4`, and the output to be stored at `output`): `cargo run -- -i input/recording.mp4 -o output -log_level debug`.

## Profiling

To perform profiling of the application, install `flamegraph` (`cargo install flamegraph`) and run it with the profiling profile.

Example: `cargo flamegraph --profile profiling -- -i input/recording.mp4 -o output`.

For memory profiling, I am using `dhat`. Run it with the profiling profile.

Example: `cargo run --profile profiling --features dhat-heap -- -i input/recording.mp4 -o output`.
