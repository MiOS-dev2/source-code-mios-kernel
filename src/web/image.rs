use alloc::vec::Vec;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;

pub struct Image {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
}

const MAX_SIDE: usize = 4096;

fn opts() -> DecoderOptions {
    DecoderOptions::default().set_max_width(MAX_SIDE).set_max_height(MAX_SIDE).png_set_strip_to_8bit(true)
}

pub fn decode(data: &[u8]) -> Option<Image> {
    if data.len() > 8 && &data[..8] == b"\x89PNG\r\n\x1a\n" { return decode_png(data); }
    if data.len() > 3 && data[0] == 0xFF && data[1] == 0xD8 { return decode_jpeg(data); }
    if data.len() > 54 && &data[..2] == b"BM" { return decode_bmp(data); }
    None
}

fn pack(buf: &[u8], w: usize, h: usize, cs: ColorSpace) -> Option<Image> {
    let n = w * h;
    let ch = cs.num_components();
    if buf.len() < n * ch { return None; }
    let mut px = Vec::with_capacity(n);
    for i in 0..n {
        let p = &buf[i * ch..i * ch + ch];
        let v = match cs {
            ColorSpace::RGB => 0xFF00_0000 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32,
            ColorSpace::RGBA => (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32,
            ColorSpace::Luma => { let g = p[0] as u32; 0xFF00_0000 | g << 16 | g << 8 | g }
            ColorSpace::LumaA => { let g = p[0] as u32; (p[1] as u32) << 24 | g << 16 | g << 8 | g }
            ColorSpace::BGR => 0xFF00_0000 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32,
            ColorSpace::BGRA => (p[3] as u32) << 24 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32,
            _ => return None,
        };
        px.push(v);
    }
    Some(Image { w: w as i32, h: h as i32, px })
}

fn decode_png(data: &[u8]) -> Option<Image> {
    let mut d = zune_png::PngDecoder::new_with_options(data, opts());
    d.decode_headers().ok()?;
    let (w, h) = d.get_dimensions()?;
    if w * h > 8_000_000 { return None; }
    let cs = d.get_colorspace()?;
    match d.decode().ok()? {
        zune_core::result::DecodingResult::U8(buf) => pack(&buf, w, h, cs),
        _ => None,
    }
}

fn decode_jpeg(data: &[u8]) -> Option<Image> {
    let mut d = zune_jpeg::JpegDecoder::new_with_options(data, opts().jpeg_set_out_colorspace(ColorSpace::RGB));
    d.decode_headers().ok()?;
    if let Some((w, h)) = d.dimensions() { if w * h > 8_000_000 { return None; } }
    let buf = d.decode().ok()?;
    let (w, h) = d.dimensions()?;
    let cs = d.get_output_colorspace().unwrap_or(ColorSpace::RGB);
    pack(&buf, w, h, cs)
}

fn rd32(d: &[u8], o: usize) -> u32 { u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) }

fn decode_bmp(d: &[u8]) -> Option<Image> {
    let off = rd32(d, 10) as usize;
    let w = rd32(d, 18) as i32;
    let hh = rd32(d, 22) as i32;
    let bpp = u16::from_le_bytes([d[28], d[29]]);
    if w <= 0 || w as usize > MAX_SIDE || hh == 0 || (bpp != 24 && bpp != 32) { return None; }
    let h = hh.abs();
    let bytes = bpp as usize / 8;
    let row = (w as usize * bytes + 3) & !3;
    let mut px = alloc::vec![0u32; (w * h) as usize];
    for y in 0..h as usize {
        let sy = if hh > 0 { h as usize - 1 - y } else { y };
        let base = off + sy * row;
        if base + w as usize * bytes > d.len() { return None; }
        for x in 0..w as usize {
            let p = &d[base + x * bytes..];
            px[y * w as usize + x] = 0xFF00_0000 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32;
        }
    }
    Some(Image { w, h, px })
}
