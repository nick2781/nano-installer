use image::ImageEncoder;
use std::path::PathBuf;

const ICON_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

pub fn embed_tool_icon(
    brand_png: &[u8],
    brand_png_path: &str,
    icon_file_name: &str,
    description: &str,
    original_file_name: &str,
) {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let icon_path = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join(icon_file_name);
    std::fs::write(&icon_path, make_tool_icon(brand_png)).expect("write nano-installer tool icon");
    let mut resource = winres::WindowsResource::new();
    resource
        .set_icon(icon_path.to_str().unwrap())
        .set("ProductName", "nano-installer")
        .set("FileDescription", description)
        .set("InternalName", original_file_name.trim_end_matches(".exe"))
        .set("OriginalFilename", original_file_name)
        .set("CompanyName", "nano-installer contributors")
        .set(
            "LegalCopyright",
            "Copyright (c) nano-installer contributors",
        );
    resource.compile().expect("embed nano-installer tool resources");
    println!("cargo:rerun-if-changed=../../build-support/windows_tool_icon.rs");
    println!("cargo:rerun-if-changed={brand_png_path}");
}

fn make_tool_icon(brand_png: &[u8]) -> Vec<u8> {
    let source = image::load_from_memory_with_format(brand_png, image::ImageFormat::Png)
        .expect("decode nano-installer branding")
        .into_rgba8();
    assert_eq!(source.dimensions(), (512, 512), "tool branding must be square");

    let frames = ICON_SIZES.map(|size| {
        let resized = image::imageops::resize(
            &source,
            size,
            size,
            image::imageops::FilterType::Lanczos3,
        );
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(resized.as_raw(), size, size, image::ExtendedColorType::Rgba8)
            .expect("encode tool icon frame");
        png
    });

    let directory_size = 6 + frames.len() * 16;
    let mut icon = Vec::with_capacity(directory_size + frames.iter().map(Vec::len).sum::<usize>());
    icon.extend_from_slice(&0u16.to_le_bytes());
    icon.extend_from_slice(&1u16.to_le_bytes());
    icon.extend_from_slice(&(frames.len() as u16).to_le_bytes());

    let mut offset = u32::try_from(directory_size).expect("ICO directory size fits u32");
    for (size, frame) in ICON_SIZES.iter().zip(&frames) {
        let dimension = if *size == 256 { 0 } else { *size as u8 };
        icon.extend_from_slice(&[dimension, dimension, 0, 0]);
        icon.extend_from_slice(&1u16.to_le_bytes());
        icon.extend_from_slice(&32u16.to_le_bytes());
        let frame_size = u32::try_from(frame.len()).expect("ICO frame size fits u32");
        icon.extend_from_slice(&frame_size.to_le_bytes());
        icon.extend_from_slice(&offset.to_le_bytes());
        offset = offset.checked_add(frame_size).expect("ICO offset fits u32");
    }
    for frame in frames {
        icon.extend_from_slice(&frame);
    }
    icon
}
