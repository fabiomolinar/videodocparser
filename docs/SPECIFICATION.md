# VideoDocParser - Application Specification

## 1. Overview

**VideoDocParser** is a command-line tool that converts video recordings of documents (e.g., PDFs, Word files, or printed text) into searchable digital formats. The application extracts text, images, and tables while preserving the logical structure of the original content. The output documents are suitable for indexing, searching, and archiving.

### Primary Use Cases

- **Digitizing lectures**: Extract slides and notes from recorded presentations.
- **Archiving paper records**: Convert recorded scans of physical documents into searchable files.
- **Extracting data**: Capture structured data from tables or forms recorded on video.
- **Accessibility**: Provide machine-readable content for visually impaired users from video sources.

---

## 2. Functional Requirements

### Input Formats

- **Video container formats**: MP4
- **Video codecs**: H.264
- **Resolution**: Minimum 480p, recommended ≥1080p for accurate OCR
- **Audio**: Ignored (not relevant for text/image extraction)

### Output Formats

- **Text-based formats**: Markdown (`.md`) + embedded image and table (when stored as images) references
- **Document formats**: PDF (searchable)
- **Metadata**: JSON index containing timestamps, extracted entities, and classification

### Core Features

- **Frame extraction and selection**
  - Extract frames based on frame-to-frame comparison to detect significant visual changes (e.g., scene changes, page turns, or slide transitions).
  - Duplicate frame detection and removal to reduce redundancy.
  - Duplicated content detection and removal within different frames to reduce redundancy.
- **OCR (Optical Character Recognition)**
  - Extracts text from selected frames using a Rust OCR library (e.g., wrapping Tesseract bindings in Rust).
  - Supports multiple languages.
- **Image detection and extraction**
  - Detects diagrams, charts, tables, or figures and stores them as separate files.
  - Embeds references to images in hybrid output documents.
  - Inserts diagrams, charts, tables, or figures stored in separate files directly into the PDF output documents.
  - Saved as PNG files and referenced in the hybrid output.
- **Table detection and structured extraction**
  - Identifies tables using heuristic methods.
  - Tables are either:
    - Reconstructed as Markdown tables (if simple and if selected output is a hybrid document), or
    - Preserved as cropped images when structure is too complex or if output is a PDF file.
- **Searchable indexing**
  - Creates metadata index for extracted text with timestamps and page references.
  - When using PDFs as output, instead of an external JSON index, the searchable text is embedded directly into PDF file.
  - Outputs index in JSON for integration with search engines.

### CLI Arguments and Options

```bash
videodocparser --input INPUT_FILE --output OUTPUT_DIR [options]
```

**Options:**

- `--input, -i`: Path to input video file
- `--output, -o`: Output directory
- `--format, -f`: Output format (`pdf`, `md`, `json`)
- `--sensitivity, -s`: Frame-to-frame sensitivity threshold
- `--lang, -l`: OCR language (default: `eng`)
- `--index`: Generate searchable JSON index
- `--log-level`: Logging verbosity (`info`, `debug`, `error`)

### Error Handling and Logging

- Input validation (unsupported video format, missing file)
- Graceful fallback if OCR fails on a frame (skip with warning)
- Detailed logs in `videodocparser.log`
- Exit codes:

  - `0`: Success
  - `1`: Invalid input
  - `2`: Processing error
  - `3`: Output failure

---

## 3. Non-Functional Requirements

- **Performance**: Process at least 1 minute of 1080p video per 10 seconds on a standard CPU; GPU acceleration preferred
- **Scalability**: Capable of processing videos up to 2 hours long with incremental output writing
- **Accuracy**:
  - OCR text accuracy ≥ 90% under good video quality
  - Table recognition accuracy ≥ 80% with well-structured tables
- **Compatibility**: Runs on Linux, Windows, and macOS
- **Resource usage**:
  - Supports GPU acceleration (CUDA, OpenCL)
  - Configurable memory buffer for long videos

---

## 4. System Architecture

### High-Level Architecture (Textual Description)

1. **Video Processing Module**: Uses FFmpeg/OpenCV to decode video frames
2. **Frame Extraction and Selection**: Frame-to-frame comparison with perceptual hashing
3. **OCR Engine**: Extracts text from frames
4. **Image & Table Recognition**: Identifies and extracts figures/tables
5. **Document Builder**: Reconstructs document structure and exports to chosen format
6. **Search Indexer**: Builds JSON index for fast searching

### Components

- **Video Processor**: Handles video decoding and frame extraction
- **Frame Analyzer**: Detects redundancy and selects representative frames
- **OCR Engine Integration**: Tesseract or EasyOCR wrapper
- **Image/Table Extractor**: Detects and saves visual elements
- **Document Builder**: Uses `printpdf`, or `pdf-writer` for PDF output, `image`, `img_hash`, `imageproc` for frame/table handling.
- **Indexer**: Generates metadata for search

---

## 5. Dependencies and Integrations

The goal is to implement **VideoDocParser** primarily in **Rust**, relying on native Rust crates where possible, and using well-maintained C libraries only when absolutely necessary (via FFI or bindings).

### Core Dependencies

#### Video Processing

- **FFmpeg via Rust bindings** (`ffmpeg-next` crate)
  - Used for decoding video frames.
  - Handles MP4 (H.264/H.265).
  - Provides frame extraction for visual comparison.

#### Frame Comparison

- **Rust image crates**:
  - [`image`](https://crates.io/crates/image) – general-purpose image processing.
  - [`img_hash`](https://crates.io/crates/img_hash) – perceptual hashing for frame difference detection.
  - Used to detect “significant change” frames (page turn, new slide).

#### OCR

- **Tesseract OCR via Rust bindings** (`tesseract-rs` crate)
  - Provides text recognition for multiple languages.
  - Chosen for stability and wide adoption.
  - No AI/ML models beyond standard OCR.

#### Document Generation

- **PDF**: [`printpdf`](https://crates.io/crates/printpdf) or [`pdf-writer`](https://crates.io/crates/pdf-writer)
  - To embed searchable text + images.
- **Markdown**: Plain text file output (no external dependency).

#### Tables

- Implemented with **heuristic methods** in Rust:
  - Line/whitespace detection via `imageproc` (Rust image processing crate).
  - Output either as Markdown tables or cropped images.

---

### Optional/Utility Dependencies

- **Logging**: `env_logger` or `tracing` for structured logs.
- **CLI**: `clap` or `structopt` for command-line argument parsing.
- **Configuration**: `serde` + `toml` (if config file support is needed).

---

### External Tools (Fallbacks)

- **Tesseract** binary (if Rust bindings are insufficient) – required runtime dependency.
- **FFmpeg** (CLI fallback) – in case video decoding via `ffmpeg-next` crate is not robust enough for certain formats.

---

## 6. User Workflow

1. User records a document on video (lecture, scan, etc.)
2. Runs the tool with input and output parameters
3. Video frames are extracted, analyzed, and passed to OCR
4. Images and tables are identified and extracted
5. Document is reconstructed and output in requested format
6. JSON index is generated (if enabled)

### Example CLI Commands

```bash
# Extract to Markdown
videodocparser -i lecture.mp4 -o output --format md

# Extract to PDF
videodocparser -i report.mp4 -o output --format pdf

# Use OCR in Spanish with lower sampling rate
videodocparser -i contrato.mp4 -o output --format pdf --lang spa
```

---

## 7. Output Document Structure

- **Text**: Extracted paragraphs with preserved ordering
- **Images**: Saved as `img_001.png`, embedded as references
- **Tables**:
  - Markdown tables in `.md` output
  - Embedded native tables in `.docx`/`.pdf`
  - Separate `.csv` exports for structured data
- **Searchable Metadata**:
  - Timestamps of frames
  - Page-like segmentation for reconstructed documents
  - Category tags: `text`, `image`, `table`
**Example Markdown Output Snippet**

```markdown
# Extracted Document

## Page 1 (00:00:15)

This is the first line of text extracted from the video.

![Figure 1](images/img_001.png)

| Name  | Value |
|-------|-------|
| Foo   | 123   |
| Bar   | 456   |
```

---

## 8. Testing and Validation

- **Unit Tests**:

  - Video frame extraction (correct number of frames)
  - OCR text extraction (sample image vs ground truth)
  - Table parsing (synthetic datasets)
- **Integration Tests**:

  - Full video-to-document pipeline
  - Error recovery (corrupted frame, missing output dir)
- **Benchmarks**:

  - OCR precision/recall against benchmark datasets
  - Table parsing accuracy on ICDAR datasets
- **Validation Data**:

  - Public OCR datasets (IAM, ICDAR)
  - Custom synthetic video datasets

---

## 9. Future Extensions

- **Handwriting Recognition**: Integrate ML models for handwritten text
- **Multi-language OCR**: Automatic language detection and mixed-language support
- **Cloud Deployment**: Batch processing service with distributed workers
- **GUI Wrapper**: Cross-platform desktop app for non-technical users
- **Audio-to-text Sync**: Combine OCR with speech-to-text for lecture videos

---

## Appendix

### Glossary

- **OCR**: Optical Character Recognition, extracting text from images
- **Frame Sampling**: Selecting representative frames from a video
- **Table Recognition**: Identifying tabular structures in images
- **Indexing**: Building a metadata database for fast search

### Example Input/Output

**Input**: 2-minute MP4 video of a scanned PDF

**Output**:

```
output/
 ├── document.pdf
 ├── images/
 │    └── img_001.png
 ├── tables/
 │    └── table_001.csv
 └── index.json
```


