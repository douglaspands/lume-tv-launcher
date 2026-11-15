//! PNG output for screenshots.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Reads a PNG file as RGB (alpha composed over black), 8 bits a channel.
pub fn read_rgb(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size().ok_or("the PNG is too large")?];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let pixels = &buffer[..info.buffer_size()];
    let over_black = |v: u8, a: u8| ((u16::from(v) * u16::from(a) + 127) / 255) as u8;
    let rgb = match info.color_type {
        png::ColorType::Rgb => pixels.to_vec(),
        png::ColorType::Rgba => pixels
            .chunks(4)
            .flat_map(|p| [0, 1, 2].map(|c| over_black(p[c], p[3])))
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g]).collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks(2)
            .flat_map(|p| {
                let g = over_black(p[0], p[1]);
                [g, g, g]
            })
            .collect(),
        other => return Err(format!("unsupported PNG color type {other:?}")),
    };
    Ok((info.width, info.height, rgb))
}

/// Writes tightly packed RGBA8 rows (top row first) as a PNG file.
pub fn write_rgba(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}
