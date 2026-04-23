use std::io::Cursor;
use std::path::Path;

use base64::Engine as _;
use crab_core::message::{ContentBlock, ImageSource, Message};
use image::{ImageFormat, ImageReader};

const MAX_FILE_SIZE: u64 = 100 * 1024; // 100 KB
const MAX_IMAGE_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
const RESIZE_TARGET_BYTES: u64 = 4 * 1024 * 1024; // headroom below the 5 MB wire limit
const JPEG_QUALITY: u8 = 85;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/// Expand `@path` references in user input into content blocks.
///
/// Scans for `@<path>` tokens in the text. For each one found:
/// - If the file exists and is small enough, its content is injected as a text
///   block (or image block for image files).
/// - If the file is too large or binary, a warning is injected instead.
/// - Unresolved references are left as-is in the text.
///
/// Returns a `Message` with role `User` containing text blocks and any
/// injected file content.
pub fn expand_at_mentions(input: &str, working_dir: &Path) -> Message {
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut remaining = input;
    let mut text_buf = String::new();

    while let Some(at_pos) = remaining.find('@') {
        text_buf.push_str(&remaining[..at_pos]);

        let after_at = &remaining[at_pos + 1..];
        if let Some(path_str) = extract_path(after_at) {
            let consumed = path_str.len();
            let resolved = working_dir.join(path_str);

            if resolved.is_file() {
                if !text_buf.is_empty() {
                    blocks.push(ContentBlock::text(std::mem::take(&mut text_buf)));
                }
                blocks.push(expand_file(&resolved, path_str));
                remaining = &after_at[consumed..];
                continue;
            }
        }

        // Not a valid file reference — keep the '@' literal
        text_buf.push('@');
        remaining = after_at;
    }

    text_buf.push_str(remaining);
    if !text_buf.is_empty() {
        blocks.push(ContentBlock::text(text_buf));
    }

    if blocks.is_empty() {
        blocks.push(ContentBlock::text(String::new()));
    }

    Message::new(crab_core::message::Role::User, blocks)
}

fn extract_path(s: &str) -> Option<&str> {
    if s.is_empty() || s.starts_with(char::is_whitespace) {
        return None;
    }
    let end = s
        .find(|c: char| c.is_whitespace() || c == '@')
        .unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    Some(&s[..end])
}

fn expand_file(path: &Path, display_path: &str) -> ContentBlock {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return expand_image(path, display_path);
    }

    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return ContentBlock::text(format!("[Error reading @{display_path}: {e}]"));
        }
    };

    if meta.len() > MAX_FILE_SIZE {
        return ContentBlock::text(format!(
            "[File @{display_path} is too large ({} KB, limit {} KB)]",
            meta.len() / 1024,
            MAX_FILE_SIZE / 1024,
        ));
    }

    match std::fs::read_to_string(path) {
        Ok(content) => ContentBlock::text(format!(
            "<file path=\"{display_path}\">\n{content}\n</file>"
        )),
        Err(_) => ContentBlock::text(format!(
            "[File @{display_path} appears to be binary and cannot be displayed]"
        )),
    }
}

fn expand_image(path: &Path, display_path: &str) -> ContentBlock {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return ContentBlock::text(format!("[Error reading @{display_path}: {e}]"));
        }
    };

    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return ContentBlock::text(format!("[Error reading @{display_path}: {e}]"));
        }
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();

    let (bytes, media_type) = if meta.len() > MAX_IMAGE_SIZE {
        match resize_image(&data, RESIZE_TARGET_BYTES) {
            Ok(result) => result,
            Err(e) => {
                return ContentBlock::text(format!(
                    "[Image @{display_path} is too large ({} KB) and could not be resized: {e}]",
                    meta.len() / 1024,
                ));
            }
        }
    } else {
        let media_type = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            _ => "image/png",
        };
        (data, media_type.to_string())
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    ContentBlock::Image {
        source: ImageSource::base64(media_type, encoded),
    }
}

/// Decode, downscale, and re-encode an oversized image so its serialized
/// payload fits under `max_bytes`.
///
/// Returns the encoded bytes and their media type. Uses JPEG (quality 85)
/// unless the source has an alpha channel, in which case PNG is retained to
/// preserve transparency.
fn resize_image(data: &[u8], max_bytes: u64) -> Result<(Vec<u8>, String), String> {
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| format!("format detection failed: {e}"))?;
    let img = reader.decode().map_err(|e| format!("decode failed: {e}"))?;

    let has_alpha = img.color().has_alpha();
    let (width, height) = (img.width(), img.height());
    let input_len = data.len() as u64;

    // Start from the observed compression ratio, then shrink geometrically if
    // the encoded output still overshoots. 8 passes is enough — each pass drops
    // dimensions by 30% so area drops ~2x per iteration.
    let initial_scale = ((max_bytes as f64) / (input_len as f64)).sqrt().min(1.0);
    let mut scale = initial_scale.max(0.05);

    for _ in 0..8 {
        let new_w = scale_dim(width, scale);
        let new_h = scale_dim(height, scale);
        let resized = img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);

        let (bytes, media_type) = if has_alpha {
            let mut buf = Cursor::new(Vec::new());
            resized
                .write_to(&mut buf, ImageFormat::Png)
                .map_err(|e| format!("PNG encode failed: {e}"))?;
            (buf.into_inner(), "image/png".to_string())
        } else {
            let rgb = resized.to_rgb8();
            let mut buf = Vec::new();
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
            encoder
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("JPEG encode failed: {e}"))?;
            (buf, "image/jpeg".to_string())
        };

        if (bytes.len() as u64) <= max_bytes {
            return Ok((bytes, media_type));
        }
        scale *= 0.7;
    }

    Err("image still exceeds size budget after 8 resize passes".to_string())
}

#[allow(clippy::cast_sign_loss)]
fn scale_dim(dim: u32, scale: f64) -> u32 {
    // scaled is clamped to >= 1.0, so the cast to u32 is always safe.
    let scaled = (f64::from(dim) * scale).round().max(1.0);
    if scaled >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        scaled as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_mentions_passes_through() {
        let msg = expand_at_mentions("hello world", Path::new("."));
        assert_eq!(msg.content.len(), 1);
        assert_eq!(msg.text(), "hello world");
    }

    #[test]
    fn unresolved_mention_kept_as_text() {
        let msg = expand_at_mentions("see @nonexistent_file_xyz.txt", Path::new("."));
        assert_eq!(msg.content.len(), 1);
        assert!(msg.text().contains("@nonexistent_file_xyz.txt"));
    }

    #[test]
    fn expands_real_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.rs"), "fn main() {}").unwrap();

        let msg = expand_at_mentions("check @test.rs please", dir.path());
        assert!(msg.content.len() >= 2);
        let full_text: String = msg
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(full_text.contains("fn main()"));
        assert!(full_text.contains("please"));
    }

    #[test]
    fn rejects_large_file() {
        let dir = tempfile::tempdir().unwrap();
        let big = vec![b'x'; (MAX_FILE_SIZE + 1) as usize];
        std::fs::write(dir.path().join("big.txt"), &big).unwrap();

        let msg = expand_at_mentions("read @big.txt", dir.path());
        assert!(msg.text().contains("too large"));
    }

    #[test]
    fn multiple_mentions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(dir.path().join("b.txt"), "bbb").unwrap();

        let msg = expand_at_mentions("@a.txt and @b.txt", dir.path());
        let text: String = msg
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("aaa"));
        assert!(text.contains("bbb"));
    }

    #[test]
    fn email_not_treated_as_mention() {
        let msg = expand_at_mentions("user@example.com", Path::new("."));
        assert_eq!(msg.content.len(), 1);
        // The @example.com part won't resolve as a file, so the whole thing passes through
        assert!(msg.text().contains("user@example.com"));
    }

    #[test]
    fn at_end_of_input() {
        let msg = expand_at_mentions("trailing @", Path::new("."));
        assert_eq!(msg.text(), "trailing @");
    }

    fn encode_png(img: &image::RgbaImage) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    fn encode_jpeg(img: &image::RgbImage) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 95);
        encoder
            .encode(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        buf
    }

    #[test]
    fn small_image_passes_through_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let img = image::RgbImage::from_fn(16, 16, |x, y| image::Rgb([x as u8, y as u8, 128]));
        let path = dir.path().join("small.png");
        image::DynamicImage::ImageRgb8(img).save(&path).unwrap();
        let original_bytes = std::fs::read(&path).unwrap();

        let msg = expand_at_mentions("@small.png", dir.path());
        let block = msg
            .content
            .iter()
            .find(|b| matches!(b, ContentBlock::Image { .. }))
            .expect("image block");

        match block {
            ContentBlock::Image { source } => {
                assert_eq!(source.media_type, "image/png");
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&source.data)
                    .unwrap();
                assert_eq!(
                    decoded, original_bytes,
                    "small image must not be re-encoded"
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn oversized_image_gets_resized() {
        let dir = tempfile::tempdir().unwrap();
        // Build a JPEG whose file size exceeds MAX_IMAGE_SIZE. Use pseudo-random
        // noise so the encoder can't compress it away.
        let w: u32 = 4096;
        let h: u32 = 4096;
        let mut img = image::RgbImage::new(w, h);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r_ch = ((x.wrapping_mul(2_654_435_761)) ^ y.wrapping_mul(40_503)) as u8;
            let g_ch = ((x.wrapping_mul(374_761_393)) ^ y.wrapping_mul(668_265_263)) as u8;
            let b_ch = ((x ^ y).wrapping_mul(2_246_822_519)) as u8;
            *pixel = image::Rgb([r_ch, g_ch, b_ch]);
        }
        let jpeg_bytes = encode_jpeg(&img);
        assert!(
            jpeg_bytes.len() as u64 > MAX_IMAGE_SIZE,
            "test fixture must exceed {} bytes, got {}",
            MAX_IMAGE_SIZE,
            jpeg_bytes.len()
        );

        let path = dir.path().join("big.jpg");
        std::fs::write(&path, &jpeg_bytes).unwrap();

        let msg = expand_at_mentions("@big.jpg", dir.path());
        let block = msg
            .content
            .iter()
            .find(|b| matches!(b, ContentBlock::Image { .. }))
            .expect("resized image should yield an Image block, not an error text");

        match block {
            ContentBlock::Image { source } => {
                // No alpha channel → should be re-encoded as JPEG
                assert_eq!(source.media_type, "image/jpeg");
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&source.data)
                    .unwrap();
                assert!(
                    (decoded.len() as u64) <= RESIZE_TARGET_BYTES,
                    "resized payload {} must be <= target {}",
                    decoded.len(),
                    RESIZE_TARGET_BYTES
                );
                assert!(
                    decoded.len() < jpeg_bytes.len(),
                    "resized output must be smaller than input"
                );
                let reloaded = ImageReader::new(Cursor::new(&decoded))
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();
                assert!(reloaded.width() > 0 && reloaded.height() > 0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn oversized_png_with_alpha_stays_png() {
        let w: u32 = 2048;
        let h: u32 = 2048;
        let mut img = image::RgbaImage::new(w, h);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r_ch = ((x.wrapping_mul(2_654_435_761)) ^ y.wrapping_mul(40_503)) as u8;
            let g_ch = ((x.wrapping_mul(374_761_393)) ^ y.wrapping_mul(668_265_263)) as u8;
            let b_ch = ((x ^ y).wrapping_mul(2_246_822_519)) as u8;
            *pixel = image::Rgba([r_ch, g_ch, b_ch, 200]);
        }
        let png_bytes = encode_png(&img);
        if (png_bytes.len() as u64) <= MAX_IMAGE_SIZE {
            // PNG compression is data-dependent; if the fixture happens to be
            // under the threshold we can't exercise the resize path, so skip.
            return;
        }

        let (bytes, media_type) = resize_image(&png_bytes, RESIZE_TARGET_BYTES).unwrap();
        assert_eq!(media_type, "image/png");
        assert!((bytes.len() as u64) <= RESIZE_TARGET_BYTES);
    }
}
