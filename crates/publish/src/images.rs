//! Lossless metadata stripping.
//!
//! Both formats are containers: the pixel data sits in segments/chunks next to
//! headers that may carry GPS, camera model, author, software and timestamps.
//! We drop the identifying parts and copy everything else byte for byte, so the
//! picture itself is unchanged — no decode, no re-encode, no generation loss.
//!
//! Not covered: EXIF `Orientation`. The viewer ignores it today, so dropping it
//! changes nothing in practice; if orientation support is ever added, the tag
//! has to be lifted out and passed along instead of discarded.

pub struct Stripped {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn strip(bytes: &[u8]) -> Result<Stripped, String> {
    if bytes.starts_with(&PNG_SIG) {
        png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        jpeg(bytes)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        webp(bytes)
    } else if bytes.len() >= 6 && (&bytes[0..6] == b"GIF89a" || &bytes[0..6] == b"GIF87a") {
        gif(bytes)
    } else {
        Err("只认 PNG / JPEG / WebP / GIF".to_string())
    }
}

const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Chunks that can name a person, a machine or a moment.
const PNG_DROP: [&[u8; 4]; 5] = [b"tEXt", b"iTXt", b"zTXt", b"tIME", b"eXIf"];

fn png(bytes: &[u8]) -> Result<Stripped, String> {
    if bytes.len() < 8 + 25 {
        return Err("PNG 太短".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&PNG_SIG);
    let mut pos = 8;
    let (mut width, mut height) = (0u32, 0u32);

    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        let kind: [u8; 4] = [bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]];
        let total = 12 + len;
        if pos + total > bytes.len() {
            return Err("PNG 数据被截断".to_string());
        }
        if &kind == b"IHDR" {
            if len < 8 {
                return Err("PNG 的 IHDR 损坏".to_string());
            }
            width = u32::from_be_bytes([
                bytes[pos + 8],
                bytes[pos + 9],
                bytes[pos + 10],
                bytes[pos + 11],
            ]);
            height = u32::from_be_bytes([
                bytes[pos + 12],
                bytes[pos + 13],
                bytes[pos + 14],
                bytes[pos + 15],
            ]);
        }
        if !PNG_DROP.contains(&&kind) {
            // Whole chunk, CRC included: dropping neighbours cannot invalidate it.
            out.extend_from_slice(&bytes[pos..pos + total]);
        }
        pos += total;
        if &kind == b"IEND" {
            break;
        }
    }

    if width == 0 || height == 0 {
        return Err("PNG 没有可用的 IHDR".to_string());
    }
    Ok(Stripped {
        bytes: out,
        width,
        height,
    })
}

/// APP1 = EXIF / XMP, APP12 = picture info, APP13 = IPTC (Photoshop), COM = comment.
/// APP0 (JFIF), APP2 (ICC) and APP14 (Adobe) are kept: they affect decoding.
fn jpeg_drop(marker: u8) -> bool {
    matches!(marker, 0xE1 | 0xEC | 0xED | 0xFE)
}

fn jpeg(bytes: &[u8]) -> Result<Stripped, String> {
    if bytes.len() < 4 {
        return Err("JPEG 太短".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..2]);
    let mut pos = 2;
    let (mut width, mut height) = (0u32, 0u32);

    loop {
        if pos + 2 > bytes.len() {
            return Err("JPEG 数据被截断".to_string());
        }
        if bytes[pos] != 0xFF {
            return Err("JPEG 段结构异常".to_string());
        }
        let marker = bytes[pos + 1];

        // Entropy-coded data follows SOS; everything from there on is opaque.
        if marker == 0xDA {
            out.extend_from_slice(&bytes[pos..]);
            break;
        }
        if marker == 0xD9 {
            out.extend_from_slice(&bytes[pos..]);
            break;
        }
        // Standalone markers carry no length.
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            out.extend_from_slice(&bytes[pos..pos + 2]);
            pos += 2;
            continue;
        }

        if pos + 4 > bytes.len() {
            return Err("JPEG 段被截断".to_string());
        }
        let seg_len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > bytes.len() {
            return Err("JPEG 段长度异常".to_string());
        }
        // SOF0..SOF15 (except DHT/JPG/DAC) carry the frame size.
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            if seg_len < 7 {
                return Err("JPEG 的 SOF 段损坏".to_string());
            }
            height = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
            width = u16::from_be_bytes([bytes[pos + 7], bytes[pos + 8]]) as u32;
        }
        if !jpeg_drop(marker) {
            out.extend_from_slice(&bytes[pos..pos + 2 + seg_len]);
        }
        pos += 2 + seg_len;
    }

    if width == 0 || height == 0 {
        return Err("JPEG 里没有找到帧尺寸".to_string());
    }
    Ok(Stripped {
        bytes: out,
        width,
        height,
    })
}

/// RIFF container: `EXIF` and `XMP ` chunks are metadata, everything else is
/// image data. `VP8X` carries flags saying which of those chunks exist, so the
/// flags have to be cleared together with the chunks.
fn webp(bytes: &[u8]) -> Result<Stripped, String> {
    let mut body: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut pos = 12;
    let (mut width, mut height) = (0u32, 0u32);
    let mut vp8x_flags: Option<usize> = None;
    let mut dropped = false;

    while pos + 8 <= bytes.len() {
        let fourcc = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        let end = pos + 8 + len;
        let padded_end = end + (len & 1);
        if padded_end > bytes.len() {
            return Err("WebP 数据被截断".to_string());
        }
        let payload = &bytes[pos + 8..end];

        let keep = match fourcc {
            b"VP8 " => {
                if payload.len() >= 10 {
                    width = u16::from_le_bytes([payload[6], payload[7]]) as u32 & 0x3FFF;
                    height = u16::from_le_bytes([payload[8], payload[9]]) as u32 & 0x3FFF;
                }
                true
            }
            b"VP8L" => {
                if payload.len() >= 5 {
                    let bits = u32::from_le_bytes([
                        payload[1], payload[2], payload[3], payload[4],
                    ]);
                    width = (bits & 0x3FFF) + 1;
                    height = ((bits >> 14) & 0x3FFF) + 1;
                }
                true
            }
            b"VP8X" => {
                if payload.len() >= 10 {
                    width = 1 + ((payload[4] as u32) | (payload[5] as u32) << 8 | (payload[6] as u32) << 16);
                    height = 1 + ((payload[7] as u32) | (payload[8] as u32) << 8 | (payload[9] as u32) << 16);
                    vp8x_flags = Some(body.len() + 8);
                }
                true
            }
            b"EXIF" | b"XMP " => {
                dropped = true;
                false
            }
            _ => true,
        };
        if keep {
            body.extend_from_slice(&bytes[pos..padded_end]);
        }
        pos = padded_end;
    }

    if width == 0 || height == 0 {
        return Err("WebP 里没有找到尺寸".to_string());
    }
    if dropped {
        if let Some(at) = vp8x_flags {
            if at < body.len() {
                body[at] &= !0x0C; // clear the EXIF (0x08) and XMP (0x04) flags
            }
        }
    }

    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&body);
    Ok(Stripped {
        bytes: out,
        width,
        height,
    })
}

/// Walks one GIF sub-block chain and returns the index just past its terminator.
fn gif_sub_blocks_end(bytes: &[u8], mut pos: usize) -> Result<usize, String> {
    loop {
        let len = *bytes.get(pos).ok_or("GIF 数据被截断")? as usize;
        pos += 1;
        if len == 0 {
            return Ok(pos);
        }
        pos += len;
        if pos > bytes.len() {
            return Err("GIF 数据被截断".to_string());
        }
    }
}

/// GIF is LZW (lossless). Metadata rides in Comment extensions and in the
/// `XMP DataXMP` application extension; the latter stuffs its payload into
/// sub-blocks with a magic trailer, so it is refused rather than guessed at.
fn gif(bytes: &[u8]) -> Result<Stripped, String> {
    if bytes.len() < 13 {
        return Err("GIF 太短".to_string());
    }
    let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
    let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
    if width == 0 || height == 0 {
        return Err("GIF 尺寸为 0".to_string());
    }

    let mut out = bytes[..13].to_vec();
    let flags = bytes[10];
    let mut pos = 13;
    if flags & 0x80 != 0 {
        let gct = 3usize << ((flags & 0x07) + 1);
        if pos + gct > bytes.len() {
            return Err("GIF 调色板被截断".to_string());
        }
        out.extend_from_slice(&bytes[pos..pos + gct]);
        pos += gct;
    }

    loop {
        match *bytes.get(pos).ok_or("GIF 数据被截断")? {
            0x3B => {
                out.push(0x3B);
                break;
            }
            0x2C => {
                if pos + 10 > bytes.len() {
                    return Err("GIF 图像块被截断".to_string());
                }
                let local = bytes[pos + 9];
                let mut data = pos + 10;
                if local & 0x80 != 0 {
                    data += 3usize << ((local & 0x07) + 1);
                }
                if data >= bytes.len() {
                    return Err("GIF 调色板被截断".to_string());
                }
                let end = gif_sub_blocks_end(bytes, data + 1)?;
                out.extend_from_slice(&bytes[pos..end]);
                pos = end;
            }
            0x21 => {
                let label = *bytes.get(pos + 1).ok_or("GIF 数据被截断")?;
                if label == 0xFF
                    && pos + 14 <= bytes.len()
                    && &bytes[pos + 3..pos + 14] == b"XMP DataXMP"
                {
                    return Err(
                        "这张 GIF 带 XMP，先导出成 PNG（或去掉元数据）再发布".to_string()
                    );
                }
                let end = gif_sub_blocks_end(bytes, pos + 2)?;
                if label != 0xFE {
                    out.extend_from_slice(&bytes[pos..end]);
                }
                pos = end;
            }
            other => return Err(format!("GIF 结构异常：0x{other:02x}")),
        }
    }

    Ok(Stripped {
        bytes: out,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::strip;
    use image::{ImageFormat, RgbImage};
    fn sample() -> RgbImage {
        RgbImage::from_fn(48, 24, |x, y| {
            image::Rgb([(x * 5) as u8, (y * 10) as u8, ((x + y) * 3) as u8])
        })
    }

    fn exif_bytes() -> Vec<u8> {
        // A minimal, well-formed TIFF/EXIF header with an Artist tag.
        let tiff: Vec<u8> = vec![
            b'I', b'I', 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00, // header, IFD at 8
            0x01, 0x00, // 1 entry
            0x13, 0x02, 0x02, 0x00, // tag 0x0213 (YCbCrPositioning), ASCII
            0x05, 0x00, 0x00, 0x00, // count
            0x1A, 0x00, 0x00, 0x00, // value offset
            0x00, 0x00, 0x00, 0x00, // next IFD
            b'h', b'i', b'!', 0x00, 0x00,
        ];
        let mut app1 = vec![0xFF, 0xE1, 0, 0];
        app1.extend_from_slice(b"Exif\0\0");
        app1.extend_from_slice(&tiff);
        let len = (app1.len() - 2) as u16;
        app1[2..4].copy_from_slice(&len.to_be_bytes());
        app1
    }

    #[test]
    fn png_drops_text_and_time_but_keeps_pixels() {
        let img = sample();
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();

        // Splice an identifying tEXt chunk in after IHDR.
        let ihdr_end = 8 + 25;
        let payload: &[u8] = b"Software\0nac\x00";
        let mut text_chunk = Vec::new();
        text_chunk.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        text_chunk.extend_from_slice(b"tEXt");
        text_chunk.extend_from_slice(payload);
        text_chunk.extend_from_slice(&[0, 0, 0, 0]); // CRC is not checked on the way out
        let mut dirty = png[..ihdr_end].to_vec();
        dirty.extend_from_slice(&text_chunk);
        dirty.extend_from_slice(&png[ihdr_end..]);

        let clean = strip(&dirty).unwrap();
        assert_eq!((clean.width, clean.height), (48, 24));
        assert!(clean.bytes.len() < dirty.len());
        assert!(!clean
            .bytes
            .windows(8)
            .any(|w| w == b"Software"));

        let back = image::load_from_memory(&clean.bytes).unwrap().to_rgb8();
        assert_eq!(back.as_raw(), img.as_raw());
    }

    #[test]
    fn jpeg_drops_app1_and_keeps_pixels() {
        let img = sample();
        let mut jpeg = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            ImageFormat::Jpeg,
        )
        .unwrap();

        let mut dirty = jpeg[..2].to_vec();
        dirty.extend_from_slice(&exif_bytes());
        dirty.extend_from_slice(&jpeg[2..]);

        let clean = strip(&dirty).unwrap();
        assert_eq!((clean.width, clean.height), (48, 24));
        assert!(clean.bytes.len() < dirty.len());
        assert!(!clean.bytes.windows(4).any(|w| w == b"Exif"));

        let before = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let after = image::load_from_memory(&clean.bytes).unwrap().to_rgb8();
        assert_eq!(before.as_raw(), after.as_raw());
    }

    #[test]
    fn webp_drops_exif_chunk_and_keeps_pixels() {
        let img = sample();
        let mut webp = Vec::new();
        {
            let encoder =
                image::codecs::webp::WebPEncoder::new_lossless(std::io::Cursor::new(&mut webp));
            encoder
                .encode(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        // Splice EXIF and XMP chunks in right after the image chunk.
        let mut dirty = webp.clone();
        for (fourcc, payload) in [(b"EXIF", &b"II*\0\x08\0\0\0"[..]), (b"XMP ", &b"<xmp/>"[..])] {
            let mut chunk = Vec::new();
            chunk.extend_from_slice(fourcc);
            chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            chunk.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                chunk.push(0);
            }
            dirty.extend_from_slice(&chunk);
        }
        let size = (dirty.len() - 8) as u32;
        dirty[4..8].copy_from_slice(&size.to_le_bytes());

        let clean = strip(&dirty).unwrap();
        assert_eq!((clean.width, clean.height), (48, 24));
        assert!(clean.bytes.len() < dirty.len());
        assert!(!clean.bytes.windows(4).any(|w| w == b"EXIF"));
        assert!(!clean.bytes.windows(4).any(|w| w == b"XMP "));

        let before = image::load_from_memory(&webp).unwrap().to_rgb8();
        let after = image::load_from_memory(&clean.bytes).unwrap().to_rgb8();
        assert_eq!(before.as_raw(), after.as_raw());
    }

    #[test]
    fn gif_drops_comment_and_keeps_pixels() {
        let img = sample();
        let mut gif = Vec::new();
        {
            let mut encoder =
                image::codecs::gif::GifEncoder::new(std::io::Cursor::new(&mut gif));
            encoder
                .encode(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8)
                .unwrap();
        }

        // Insert a comment extension right after the global colour table.
        let flags = gif[10];
        let mut insert = 13;
        if flags & 0x80 != 0 {
            insert += 3usize << ((flags & 0x07) + 1);
        }
        let payload = b"Software: SecretEditor";
        let mut comment = vec![0x21, 0xFE, payload.len() as u8];
        comment.extend_from_slice(payload);
        comment.push(0);
        let mut dirty = gif[..insert].to_vec();
        dirty.extend_from_slice(&comment);
        dirty.extend_from_slice(&gif[insert..]);

        let clean = strip(&dirty).unwrap();
        assert_eq!((clean.width, clean.height), (48, 24));
        assert!(clean.bytes.len() < dirty.len());
        assert!(!clean.bytes.windows(8).any(|w| w == b"Software"));

        let before = image::load_from_memory(&gif).unwrap().to_rgb8();
        let after = image::load_from_memory(&clean.bytes).unwrap().to_rgb8();
        assert_eq!(before.as_raw(), after.as_raw());
    }

    #[test]
    fn other_formats_are_refused() {
        assert!(strip(b"BM0000000000").is_err());
        assert!(strip(b"not an image").is_err());
    }
}
