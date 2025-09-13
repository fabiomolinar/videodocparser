# videodocparser

VideoDocParser is a command-line tool that converts video recordings of documents (e.g., PDFs, Word files, or printed text) into searchable digital formats.

## External dependencies

- ffmpeg related: [notes on building](https://github.com/zmwangx/rust-ffmpeg/wiki/Notes-on-building)
- tesseract related: `cmake` [download](https://cmake.org/download/)

## Notes

- To get `tesseract-rs` working, first I had to do a `cargo build --release` to have its binaries created.