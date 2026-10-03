//! Writes one transcript per audio file as TXT, DOCX, or PDF.

use std::fs::OpenOptions;
use std::io::{Cursor, ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Txt,
    Docx,
    Pdf,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
        }
    }
}

const FALLBACK_STEM: &str = "transcript";
const MAX_STEM_CHARS: usize = 150;
const MAX_NAME_ATTEMPTS: u32 = 999;
const INVALID_NAME_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Writes `<audio stem>.<ext>` into `dir`, adding ` (n)` instead of
/// overwriting an existing file. Returns the written path.
pub fn write_transcript(
    dir: &Path,
    source_path: &str,
    format: ExportFormat,
    text: &str,
) -> Result<PathBuf> {
    let title = Path::new(source_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| FALLBACK_STEM.to_string());
    let bytes = render(format, &title, text)?;
    let stem = safe_file_stem(source_path);
    let ext = format.extension();
    for attempt in 0..=MAX_NAME_ATTEMPTS {
        let name = match attempt {
            0 => format!("{stem}.{ext}"),
            n => format!("{stem} ({n}).{ext}"),
        };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&bytes) {
                    let _ = std::fs::remove_file(&path);
                    return Err(e.into());
                }
                return Ok(path);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(AppError::invalid_input(format!(
        "Too many existing files named {stem}.{ext}"
    )))
}

fn render(format: ExportFormat, title: &str, text: &str) -> Result<Vec<u8>> {
    match format {
        ExportFormat::Txt => Ok(text.as_bytes().to_vec()),
        ExportFormat::Docx => render_docx(title, text),
        ExportFormat::Pdf => Ok(render_pdf(title, text)),
    }
}

/// A Windows-safe file stem from the audio file name.
fn safe_file_stem(source_path: &str) -> String {
    let stem = source_path
        .rsplit(['/', '\\'])
        .next()
        .map(|name| name.rsplit_once('.').map_or(name, |(stem, _)| stem))
        .unwrap_or_default();
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_control() || INVALID_NAME_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .take(MAX_STEM_CHARS)
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        return FALLBACK_STEM.to_string();
    }
    let base = trimmed.split('.').next().unwrap_or(trimmed);
    if RESERVED_NAMES.contains(&base.to_ascii_uppercase().as_str()) {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    }
}

// --- DOCX -------------------------------------------------------------------

const DOCX_CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const DOCX_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

/// Word half-points: 16 pt title.
const DOCX_TITLE_HALF_POINTS: u32 = 32;

fn render_docx(title: &str, text: &str) -> Result<Vec<u8>> {
    let mut body = format!(
        r#"<w:p><w:r><w:rPr><w:b/><w:sz w:val="{DOCX_TITLE_HALF_POINTS}"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        xml_escape(title)
    );
    for line in text.lines() {
        body.push_str(&format!(
            r#"<w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
            xml_escape(line)
        ));
    }
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
    );
    let parts = [
        ("[Content_Types].xml", DOCX_CONTENT_TYPES),
        ("_rels/.rels", DOCX_RELS),
        ("word/document.xml", document.as_str()),
    ];
    let docx_error =
        |e: zip::result::ZipError| AppError::internal(format!("DOCX write failed: {e}"));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in parts {
        zip.start_file(name, options).map_err(docx_error)?;
        zip.write_all(content.as_bytes())?;
    }
    Ok(zip.finish().map_err(docx_error)?.into_inner())
}

/// Escapes markup and drops control characters XML 1.0 forbids.
fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

// --- PDF --------------------------------------------------------------------

/// A4 in PDF points.
const PDF_PAGE_WIDTH: u32 = 595;
const PDF_PAGE_HEIGHT: u32 = 842;
const PDF_MARGIN: u32 = 56;
const PDF_BODY_SIZE: u32 = 11;
const PDF_TITLE_SIZE: u32 = 14;
const PDF_LEADING: u32 = 15;
const PDF_LINES_PER_PAGE: usize = ((PDF_PAGE_HEIGHT - 2 * PDF_MARGIN) / PDF_LEADING) as usize;
// ponytail: fixed column wrap from Helvetica's average glyph width; wide
// glyph runs (e.g. "WWWW") can overrun the margin. Upgrade to per-glyph
// AFM widths if that shows up in real transcripts.
const PDF_WRAP_COLUMNS: usize = 80;
/// First object id of the page objects; 1-4 are catalog, pages, and fonts.
const PDF_FIRST_PAGE_ID: usize = 5;

struct PdfLine {
    bold: bool,
    text: String,
}

/// Uses the built-in Helvetica with WinAnsi encoding: no font embedding,
/// covers Indonesian and English. Characters outside it print as `?`.
fn render_pdf(title: &str, text: &str) -> Vec<u8> {
    let mut lines: Vec<PdfLine> = wrap(title, PDF_WRAP_COLUMNS)
        .into_iter()
        .map(|text| PdfLine { bold: true, text })
        .collect();
    lines.push(PdfLine {
        bold: false,
        text: String::new(),
    });
    for paragraph in text.lines() {
        lines.extend(
            wrap(paragraph, PDF_WRAP_COLUMNS)
                .into_iter()
                .map(|text| PdfLine { bold: false, text }),
        );
    }
    let pages: Vec<&[PdfLine]> = lines.chunks(PDF_LINES_PER_PAGE).collect();
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", PDF_FIRST_PAGE_ID + 2 * i))
        .collect();
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes(),
        pdf_font("Helvetica"),
        pdf_font("Helvetica-Bold"),
    ];
    for (i, page) in pages.iter().enumerate() {
        let content_id = PDF_FIRST_PAGE_ID + 2 * i + 1;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PDF_PAGE_WIDTH} {PDF_PAGE_HEIGHT}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>"
        ).into_bytes());
        let stream = pdf_page_stream(page);
        let mut object = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        object.extend(stream);
        object.extend(b"\nendstream");
        objects.push(object);
    }
    serialize_pdf(&objects)
}

fn pdf_font(base: &str) -> Vec<u8> {
    format!("<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>")
        .into_bytes()
}

fn pdf_page_stream(lines: &[PdfLine]) -> Vec<u8> {
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        if line.text.is_empty() {
            continue;
        }
        let (font, size) = if line.bold {
            ("F2", PDF_TITLE_SIZE)
        } else {
            ("F1", PDF_BODY_SIZE)
        };
        let y = PDF_PAGE_HEIGHT - PDF_MARGIN - PDF_LEADING * row as u32 - PDF_BODY_SIZE;
        out.extend(format!("BT /{font} {size} Tf {PDF_MARGIN} {y} Td (").into_bytes());
        out.extend(pdf_string_bytes(&line.text));
        out.extend(b") Tj ET\n");
    }
    out
}

/// Encodes to WinAnsi and escapes PDF literal-string delimiters.
fn pdf_string_bytes(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        let byte = win_ansi_byte(c);
        if matches!(byte, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(byte);
    }
    out
}

fn win_ansi_byte(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        '\u{A0}'..='\u{FF}' => c as u32 as u8,
        '\u{20AC}' => 0x80,
        '\u{2026}' => 0x85,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201C}' => 0x93,
        '\u{201D}' => 0x94,
        '\u{2022}' => 0x95,
        '\u{2013}' => 0x96,
        '\u{2014}' => 0x97,
        _ => b'?',
    }
}

fn serialize_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).into_bytes());
        out.extend(body);
        out.extend(b"\nendobj\n");
    }
    let xref_offset = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).into_bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").into_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .into_bytes(),
    );
    out
}

/// Greedy word wrap by character count; words longer than a line are split.
fn wrap(paragraph: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current: Vec<char> = Vec::new();
    for word in paragraph.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        if !current.is_empty() && current.len() + 1 + word.len() > columns {
            lines.push(current.drain(..).collect());
        }
        while word.len() > columns {
            lines.push(word.drain(..columns).collect());
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.extend(word);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current.into_iter().collect());
    }
    lines
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("voxitype-export-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn stem_is_windows_safe() {
        assert_eq!(safe_file_stem(r"C:\audio\rapat 1.mp3"), "rapat 1");
        assert_eq!(safe_file_stem("/x/a:b?c.tar.gz"), "a_b_c.tar");
        assert_eq!(safe_file_stem(r"C:\audio\con.wav"), "_con");
        assert_eq!(safe_file_stem(r"C:\audio\...wav"), FALLBACK_STEM);
    }

    #[test]
    fn existing_file_is_never_overwritten() {
        let dir = scratch_dir("collide");
        let first = write_transcript(&dir, r"C:\a\meeting.mp3", ExportFormat::Txt, "one").unwrap();
        let second = write_transcript(&dir, r"D:\b\meeting.wav", ExportFormat::Txt, "two").unwrap();

        assert_eq!(first.file_name().unwrap(), "meeting.txt");
        assert_eq!(second.file_name().unwrap(), "meeting (1).txt");
        assert_eq!(std::fs::read_to_string(first).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(second).unwrap(), "two");
    }

    #[test]
    fn docx_holds_escaped_paragraphs() {
        let bytes = render_docx("a.mp3", "Halo & <dunia>\nbaris dua").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert!(archive.by_name("[Content_Types].xml").is_ok());
        assert!(archive.by_name("_rels/.rels").is_ok());
        let mut document = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document)
            .unwrap();

        assert!(document.contains("Halo &amp; &lt;dunia&gt;"));
        assert_eq!(document.matches("<w:p>").count(), 3);
    }

    #[test]
    fn pdf_xref_offsets_point_at_objects() {
        let long_text = "kata ".repeat(PDF_WRAP_COLUMNS * PDF_LINES_PER_PAGE / 4);
        let pdf = render_pdf("a (1).mp3", &long_text);
        // Offsets are byte positions; lossy UTF-8 decoding of the binary
        // header comment would shift them.
        let text = String::from_utf8_lossy(&pdf);
        let xref_at: usize = text
            .rsplit("startxref\n")
            .next()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();

        assert!(pdf[xref_at..].starts_with(b"xref"));
        let xref = String::from_utf8_lossy(&pdf[xref_at..]);
        let entries: Vec<usize> = xref
            .lines()
            .skip(3)
            .take_while(|l| l.ends_with(" n "))
            .map(|l| l[..10].parse().unwrap())
            .collect();
        assert_eq!(entries.len(), PDF_FIRST_PAGE_ID - 1 + 4);
        for (i, offset) in entries.iter().enumerate() {
            assert!(pdf[*offset..].starts_with(format!("{} 0 obj", i + 1).as_bytes()));
        }
        assert!(text.contains("/Count 2"), "long text spans two pages");
        assert!(text.contains(r"(a \(1\).mp3) Tj"));
    }

    #[test]
    fn pdf_maps_non_latin_to_placeholder() {
        assert_eq!(pdf_string_bytes("é\u{2019}日"), vec![0xE9, 0x92, b'?']);
    }

    #[test]
    fn wrap_splits_on_words_and_overlong_words() {
        assert_eq!(wrap("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(wrap("abcdefg", 3), vec!["abc", "def", "g"]);
        assert_eq!(wrap("", 3), vec![""]);
    }
}
