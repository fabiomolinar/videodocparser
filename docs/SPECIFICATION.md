# VideoDocParser - Application Specification

## 1. Overview

**VideoDocParser** is a command-line tool that converts video recordings of documents (e.g., PDFs, Word files, or printed text) into searchable digital formats. The application extracts text, images, and tables (as images) while preserving the logical structure of the original content. The output documents are suitable for indexing, searching, and archiving.

### Primary Use Cases

- **Digitizing lectures**: Extract slides and notes from recorded presentations.
- **Archiving paper records**: Convert recorded scans of physical documents into searchable files.
- **Extracting data**: Capture structured data from tables or forms recorded on video.
- **Accessibility**: Provide machine-readable content for visually impaired users from video sources.

---

## 2. Functional Requirements

### Input Formats

- **Video container formats**: MP4.
- **Video codecs**: H.264.
- **Resolution**: Minimum 480p, recommended ≥1080p for accurate OCR.
- **Audio**: Ignored (not relevant for text/image extraction).

### Output Formats

- **Text-based formats**: Markdown (`.md`) with embedded image references for figures and tables.
- **Document formats**: PDF (searchable, with text and embedded images/tables).
- **Images**: A folder with non-repeated images from the recorded document.
- **Metadata**: Optional JSON index containing timestamps, extracted entities, and classification.

### Core Features

- **Frame extraction and selection**
  - Extract frames based on **frame-to-frame comparison** to detect significant visual changes (scene changes, page turns, slide transitions).
  - Detect and skip duplicate frames to reduce redundancy.
- **OCR (Optical Character Recognition)**
  - Extracts text from selected frames using **Tesseract OCR** via Rust bindings.
  - Supports multiple languages.
- **Image and Table Detection**
  - Detects diagrams, charts, figures, and tables.
  - Saves all detected visual elements as **PNG images**.
  - Embeds references to these images in Markdown outputs or insert them directly into PDF outputs.
- **Searchable indexing**
  - Metadata index for extracted text with timestamps and page references.
  - Embedded searchable text in PDF output.
  - Optional JSON index for external integration.

### CLI Arguments and Options

```bash
videodocparser --input INPUT_FILE --output OUTPUT_DIR [options]
```

**Options:**

- `--input, -i`: Path to input video file
- `--output, -o`: Output directory
- `--format, -f`: Output format (`pdf`, `md`, `img`)
- `--sensitivity, -s`: Frame-to-frame sensitivity threshold
- `--lang, -l`: OCR language (default: `eng`)
- `--index`: Generate optional JSON index
- `--log-level`: Logging verbosity (`info`, `debug`, `error`)

### Error Handling and Logging

- Validate input file format and existence.
- Skip frames gracefully if OCR fails, logging a warning.
- Logs written to `videodocparser.log`.
- Exit codes:
  - `0`: Success
  - `1`: Invalid input
  - `2`: Processing error
  - `3`: Output failure

---

## 3. Non-Functional Requirements

- **Performance**: Process ≥1 minute of 1080p video per 10 seconds on a standard CPU; GPU acceleration optional.
- **Scalability**: Handles videos up to 2 hours with incremental output writing.
- **Accuracy**:
  - OCR ≥90% under good video quality.
  - Visual element (image/table) detection accuracy ≥85%.
- **Compatibility**: Linux, Windows, macOS.
- **Resource Usage**:
  - Supports GPU acceleration (CUDA/OpenCL).
  - Configurable memory buffer for long videos.

---

## 4. System Architecture

### High-Level Architecture

1. **Video Processing Module**: Uses FFmpeg via Rust bindings to decode video frames.
2. **Frame Extraction and Selection**: Performs frame-to-frame comparison with perceptual hashing.
3. **OCR Engine**: Extracts text from selected frames using Tesseract.
4. **Visual Element Detection**: Detects images, figures, charts, and tables, saving them as PNGs.
5. **Document Builder**: Reconstructs document structure and exports to Markdown or PDF, embedding images/tables.
6. **Search Indexer**: Builds embedded text in PDF and optional JSON metadata.

### Components

- **Video Processor**: Handles video decoding and frame extraction.
- **Frame Analyzer**: Performs frame comparison and selects significant frames.
- **OCR Engine Integration**: Tesseract OCR via Rust bindings.
- **Visual Element Extractor**: Detects, crops, and saves images and tables as PNGs.
- **Document Builder**: Uses `printpdf` or `pdf-writer` for PDF output; Rust image crates for handling visual elements.
- **Indexer**: Generates searchable metadata and optional JSON index.

---

## 5. Dependencies and Integrations

### Core Dependencies (Rust-First)

- **Video Processing**: `ffmpeg-next` crate (FFmpeg bindings).
- **Frame Comparison & Image Processing**: `image`, `img_hash`, `imageproc`.
- **OCR**: `tesseract-rs` crate (Tesseract OCR).
- **Document Generation**: `printpdf` or `pdf-writer` for PDF; Markdown output requires no external library.

### Optional / Utility Dependencies

- Logging: `env_logger` or `tracing`.
- CLI parsing: `clap` or `structopt`.
- Config parsing: `serde` + `toml`.

### External Tools (Fallbacks)

- Tesseract binary if Rust bindings are insufficient.
- FFmpeg CLI fallback for video decoding issues.

---

## 6. User Workflow

1. Record a document on video (lecture, scan, etc.).
2. Run VideoDocParser with input/output parameters.
3. Frames are extracted and analyzed for significant changes.
4. OCR extracts text; images, charts, figures, and tables are detected and saved as PNGs.
5. Document is reconstructed in Markdown or PDF format with references to visual elements.
6. Optional JSON metadata index is generated.

### Example CLI Commands

```bash
# Extract to Markdown
videodocparser -i lecture.mp4 -o output --format md

# Extract to PDF
videodocparser -i report.mp4 -o output --format pdf

# Use OCR in Spanish with sensitivity adjustment
videodocparser -i contrato.mp4 -o output --format pdf --lang spa --sensitivity 0.7
```

---

## 7. Output Document Structure

- **Text**: Preserves paragraph order.
- **Images & Tables**: Saved as PNG files (e.g., `img_001.png`) and embedded/referenced in output.
- **Searchable Metadata**:
  - Frame timestamps
  - Page-like segmentation
  - Category tags: `text`, `image/table`

### Example Markdown Output Snippet

```markdown
# Extracted Document

## Page 1 (00:00:15)

This is the first line of text extracted from the video.

![Figure 1](images/img_001.png)
![Table 1](images/table_001.png)
```

---

## 8. Testing and Validation

- **Unit Tests**: Frame extraction, OCR, visual element detection.
- **Integration Tests**: End-to-end video-to-document pipeline, error recovery.
- **Benchmarks**: OCR precision/recall, visual element detection accuracy.
- **Validation Data**: IAM, ICDAR datasets, synthetic video datasets.

---

## 9. Future Extensions

- Multi-language OCR (Tesseract built-in support).
- Parallel frame decoding for performance improvement.
- Improved heuristics for complex visual element detection.
- GUI wrapper around CLI for non-technical users.
- Cloud/batch processing pipeline (Rust-based).

---

## Appendix

### Glossary

- **OCR**: Optical Character Recognition.
- **Frame Comparison**: Detecting significant visual changes between frames.
- **Visual Element Detection**: Identifying images, charts, figures, and tables as separate objects.
- **Indexing**: Building searchable metadata for documents.

### Example Input/Output

**Input**: 2-minute MP4 video of a scanned PDF

**Output**:

```
output/
 ├── document.pdf
 ├── images/
 │    ├── img_001.png
 │    └── table_001.png
 └── index.json
```
