#[cfg(not(target_arch = "x86_64"))]
compile_error!("nano-installer-native-x64 must be built for x86_64");

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateRoundRectRgn, DeleteDC,
    DeleteObject, DrawTextW, EndPaint, GdiAlphaBlend, SelectObject, SetBkMode, SetTextColor,
    SetWindowRgn, UpdateWindow, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    DIB_RGB_COLORS, DT_CENTER, DT_LEFT, DT_SINGLELINE, DT_VCENTER, FW_BOLD, FW_NORMAL, HDC,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory, IWICPalette,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetSystemMetrics, LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassExW, SendMessageW,
    ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HTCAPTION, IDC_ARROW, MB_ICONERROR,
    MB_OK, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_MINIMIZE, SW_SHOW, WM_DESTROY, WM_ERASEBKGND,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT, WNDCLASSEXW,
    WS_EX_APPWINDOW, WS_POPUP,
};

const BUNDLE_MAGIC: &[u8; 8] = b"NATVRS01";
const FOOTER_MAGIC: &[u8; 8] = b"NATVEND1";
const BUNDLE_VERSION: u16 = 1;
static UI: OnceLock<RuntimeUi> = OnceLock::new();

struct NativeImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct ImageLayer {
    image: NativeImage,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

struct RuntimeUi {
    width: i32,
    height: i32,
    corner_radius: i32,
    layers: Vec<ImageLayer>,
    texts: Vec<TextLayer>,
    actions: Vec<ActionRegion>,
}

struct TextLayer {
    text: String,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    font_size: i32,
    color: COLORREF,
    bold: bool,
    centered: bool,
}

struct ActionRegion {
    action: WindowAction,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

enum WindowAction {
    Close,
    Minimize,
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub fn show_runtime_error(error: &anyhow::Error) {
    let message = HSTRING::from(format!("Native runtime failed:\n{error:#}"));
    unsafe {
        let _ = MessageBoxW(None, &message, w!("nano-installer"), MB_OK | MB_ICONERROR);
    }
}

pub fn run_runtime() -> Result<()> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let bundle = read_embedded_bundle(&exe)?.context("native resource bundle is missing")?;
    run_embedded(bundle)
}

pub fn run_builder_cli() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("build")) {
        bail!("usage: nano-installer-native-x64.exe build --project <directory> [--output <exe>]");
    }

    let mut project = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--project" => project = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            value => bail!("unsupported argument: {value}"),
        }
    }

    let project = project.context("--project is required")?;
    let output = output.unwrap_or_else(|| project.join("dist").join("Native_Setup.exe"));
    let stub_name = installer_stub_for_project(&project)?;
    let stub = find_native_stub(&stub_name)?;
    let bundle = pack_project(&project)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&stub, &output)
        .with_context(|| format!("failed to create {}", output.display()))?;
    let mut file = OpenOptions::new().append(true).open(&output)?;
    file.write_all(&bundle)?;
    file.write_all(&(bundle.len() as u64).to_le_bytes())?;
    file.write_all(FOOTER_MAGIC)?;
    file.flush()?;
    Ok(())
}

fn installer_stub_for_project(project: &Path) -> Result<String> {
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(project.join("installer_config.json"))?)?;
    let payload = config["resources"]["payload_file"]
        .as_str()
        .context("resources.payload_file is required")?;
    let signature = std::fs::read(project.join(payload))?;
    if signature.starts_with(b"PK") {
        Ok("native-zlib-x64.exe".to_string())
    } else if signature.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        Ok("native-lzma-x64.exe".to_string())
    } else {
        bail!("payload must be a ZIP or 7z archive")
    }
}

fn find_native_stub(name: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(directory) = std::env::var_os("NANO_INSTALLER_NATIVE_STUB_DIR") {
        candidates.push(PathBuf::from(directory).join(name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(directory) = exe.parent() {
            candidates.push(directory.join(name));
        }
    }
    candidates.push(PathBuf::from("target/release").join(name));
    candidates.push(PathBuf::from("../../target/release").join(name));
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .with_context(|| format!("native runtime stub not found: {name}"))
}

fn pack_project(project: &Path) -> Result<Vec<u8>> {
    let mut files = Vec::new();
    let config_path = project.join("installer_config.json");
    let config_data = std::fs::read(&config_path)
        .with_context(|| format!("missing project file: {}", config_path.display()))?;
    let config: serde_json::Value = serde_json::from_slice(&config_data)?;
    files.push(("installer_config.json".to_string(), config_data));

    for (key, default) in [
        ("layouts_dir", "layouts"),
        ("assets_dir", "assets"),
        ("locales_dir", "locales"),
    ] {
        let directory = config["resources"][key].as_str().unwrap_or(default);
        collect_directory(project, &project.join(directory), &mut files)?;
    }
    let scripts = project.join("scripts");
    if scripts.is_dir() {
        collect_directory(project, &scripts, &mut files)?;
    }
    let payload = config["resources"]["payload_file"]
        .as_str()
        .context("resources.payload_file is required")?;
    collect_file(project, &project.join(payload), &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut bundle = Vec::new();
    bundle.extend_from_slice(BUNDLE_MAGIC);
    bundle.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
    bundle.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (name, data) in files {
        let name = name.as_bytes();
        let name_len: u16 = name.len().try_into().context("bundle path is too long")?;
        bundle.extend_from_slice(&name_len.to_le_bytes());
        bundle.extend_from_slice(name);
        bundle.extend_from_slice(&(data.len() as u64).to_le_bytes());
        bundle.extend_from_slice(&data);
    }
    Ok(bundle)
}

fn collect_directory(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("missing project directory: {}", directory.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_directory(root, &entry.path(), files)?;
        } else {
            collect_file(root, &entry.path(), files)?;
        }
    }
    Ok(())
}

fn collect_file(root: &Path, path: &Path, files: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
    if !path.is_file() {
        bail!("missing project file: {}", path.display());
    }
    let name = path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/");
    files.push((name, std::fs::read(path)?));
    Ok(())
}

fn read_embedded_bundle(exe: &Path) -> Result<Option<HashMap<String, Vec<u8>>>> {
    let bytes = std::fs::read(exe)?;
    if bytes.len() < 16 || &bytes[bytes.len() - 8..] != FOOTER_MAGIC {
        return Ok(None);
    }
    let size = u64::from_le_bytes(bytes[bytes.len() - 16..bytes.len() - 8].try_into()?) as usize;
    let start = bytes
        .len()
        .checked_sub(16 + size)
        .context("invalid native bundle size")?;
    parse_bundle(&bytes[start..start + size]).map(Some)
}

fn parse_bundle(data: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
    let mut cursor = Cursor::new(data);
    let mut magic = [0u8; 8];
    cursor.read_exact(&mut magic)?;
    if &magic != BUNDLE_MAGIC {
        bail!("invalid native bundle magic");
    }
    if read_u16(&mut cursor)? != BUNDLE_VERSION {
        bail!("unsupported native bundle version");
    }
    let count = read_u32(&mut cursor)?;
    let mut files = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let name_len = read_u16(&mut cursor)? as usize;
        let mut name = vec![0u8; name_len];
        cursor.read_exact(&mut name)?;
        let size = read_u64(&mut cursor)? as usize;
        let mut contents = vec![0u8; size];
        cursor.read_exact(&mut contents)?;
        files.insert(String::from_utf8(name)?, contents);
    }
    Ok(files)
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16> {
    let mut bytes = [0; 2];
    cursor.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64> {
    let mut bytes = [0; 8];
    cursor.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn run_embedded(files: HashMap<String, Vec<u8>>) -> Result<()> {
    let ui = load_layout(&files)?;
    let (width, height) = (ui.width, ui.height);
    UI.set(ui)
        .map_err(|_| anyhow::anyhow!("native UI was already initialized"))?;
    run_window(width, height)
}

fn load_layout(files: &HashMap<String, Vec<u8>>) -> Result<RuntimeUi> {
    let config: serde_json::Value = serde_json::from_slice(
        files
            .get("installer_config.json")
            .context("installer_config.json missing from native bundle")?,
    )?;
    let layout_path = config["wizard"]["pages"][0]["layout"]
        .as_str()
        .context("wizard.pages[0].layout is missing")?;
    let xml = std::str::from_utf8(
        files
            .get(layout_path)
            .with_context(|| format!("layout missing from native bundle: {layout_path}"))?,
    )?;
    let document = roxmltree::Document::parse(xml)?;
    let page = document
        .descendants()
        .find(|node| node.has_tag_name("Page"))
        .context("layout has no Page element")?;
    let width = int_attribute(page, "width").unwrap_or(720);
    let height = int_attribute(page, "height").unwrap_or(450);
    let corner_radius = int_attribute(page, "border-radius").unwrap_or(0);
    let mut layers = Vec::new();
    let mut texts = Vec::new();
    let mut actions = Vec::new();
    let locale = std::env::var("NANO_INSTALLER_TEST_LOCALE")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            config["localization"]["default_locale"]
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_else(|| "zh-CN".to_string());
    let locales_dir = config["resources"]["locales_dir"]
        .as_str()
        .unwrap_or("locales");
    let locale_path = format!("{locales_dir}/{locale}.json");
    let translations: HashMap<String, String> = serde_json::from_slice(
        files
            .get(&locale_path)
            .with_context(|| format!("locale missing from native bundle: {locale_path}"))?,
    )?;

    if let Some(path) = page.attribute("background-image") {
        push_layer(files, &mut layers, path, 0, 0, width, height)?;
    }
    for node in page.descendants().filter(|node| node.is_element()) {
        if node
            .ancestors()
            .any(|ancestor| ancestor.attribute("visible") == Some("false"))
        {
            continue;
        }
        let left = int_attribute(node, "left").unwrap_or(0);
        let top = int_attribute(node, "top").unwrap_or(0);
        let layer_width = int_attribute(node, "width").unwrap_or(0);
        let layer_height = int_attribute(node, "height").unwrap_or(0);
        if node.has_tag_name("Button") && layer_width > 0 && layer_height > 0 {
            let action = match node.attribute("action") {
                Some("minimize") => Some(WindowAction::Minimize),
                Some("close") | Some("close_confirm") => Some(WindowAction::Close),
                _ => None,
            };
            if let Some(action) = action {
                actions.push(ActionRegion {
                    action,
                    left,
                    top,
                    right: left + layer_width,
                    bottom: top + layer_height,
                });
            }
        }
        if layer_width > 0 && layer_height > 0 {
            if let Some((text, centered)) = text_for_node(node, &locale, &translations) {
                texts.push(TextLayer {
                    text,
                    left,
                    top,
                    width: layer_width,
                    height: layer_height,
                    font_size: int_attribute(node, "font-size").unwrap_or(12),
                    color: parse_color(node.attribute("color").unwrap_or("#FFFFFFFF")),
                    bold: node.attribute("font-weight") == Some("bold"),
                    centered,
                });
            }
        }
        let source = match node.tag_name().name() {
            "Image" | "Icon" if node.attribute("position") == Some("absolute") => {
                node.attribute("src")
            }
            "Button" => node
                .attribute("normal-image")
                .and_then(image_path_from_style),
            _ => None,
        };
        if let Some(source) = source.filter(|_| layer_width > 0 && layer_height > 0) {
            push_layer(
                files,
                &mut layers,
                source,
                left,
                top,
                layer_width,
                layer_height,
            )?;
        }
    }
    Ok(RuntimeUi {
        width,
        height,
        corner_radius,
        layers,
        texts,
        actions,
    })
}

fn text_for_node(
    node: roxmltree::Node<'_, '_>,
    locale: &str,
    translations: &HashMap<String, String>,
) -> Option<(String, bool)> {
    let raw = match node.tag_name().name() {
        "Button" | "Label" => node.attribute("text").or_else(|| node.attribute("value"))?,
        "Select" => node
            .children()
            .find(|option| {
                option.has_tag_name("Option") && option.attribute("value") == Some(locale)
            })
            .or_else(|| node.children().find(|option| option.has_tag_name("Option")))?
            .attribute("text")?,
        _ => return None,
    };
    let text = raw
        .strip_prefix('@')
        .and_then(|key| translations.get(key))
        .cloned()
        .unwrap_or_else(|| raw.to_string());
    let centered = node.has_tag_name("Button") || node.attribute("textalign") == Some("center");
    Some((text, centered))
}

fn parse_color(value: &str) -> COLORREF {
    let hex = value.trim_start_matches('#');
    let rgb = if hex.len() == 8 { &hex[2..] } else { hex };
    let parsed = u32::from_str_radix(rgb, 16).unwrap_or(0xFFFFFF);
    let red = (parsed >> 16) & 0xFF;
    let green = (parsed >> 8) & 0xFF;
    let blue = parsed & 0xFF;
    COLORREF(red | (green << 8) | (blue << 16))
}

fn image_path_from_style(value: &str) -> Option<&str> {
    value
        .split("file='")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .or(Some(value))
}

fn int_attribute(node: roxmltree::Node<'_, '_>, name: &str) -> Option<i32> {
    node.attribute(name)?.parse().ok()
}

fn push_layer(
    files: &HashMap<String, Vec<u8>>,
    layers: &mut Vec<ImageLayer>,
    source: &str,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> Result<()> {
    let encoded = files
        .get(source)
        .with_context(|| format!("layout asset missing from native bundle: {source}"))?;
    layers.push(ImageLayer {
        image: decode_image(encoded)?,
        left,
        top,
        width,
        height,
    });
    Ok(())
}

fn decode_image(encoded: &[u8]) -> Result<NativeImage> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let _guard = ComGuard;
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = factory.CreateStream()?;
        stream.InitializeFromMemory(encoded)?;
        let decoder = factory.CreateDecoderFromStream(
            &stream,
            std::ptr::null(),
            WICDecodeMetadataCacheOnLoad,
        )?;
        let frame = decoder.GetFrame(0)?;
        let converter = factory.CreateFormatConverter()?;
        converter.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None::<&IWICPalette>,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let mut width = 0;
        let mut height = 0;
        converter.GetSize(&mut width, &mut height)?;
        let stride = width.checked_mul(4).context("bitmap stride overflow")?;
        let size = stride.checked_mul(height).context("bitmap size overflow")? as usize;
        let mut pixels = vec![0; size];
        converter.CopyPixels(std::ptr::null(), stride, &mut pixels)?;
        Ok(NativeImage {
            width,
            height,
            pixels,
        })
    }
}

fn run_window(client_width: i32, client_height: i32) -> Result<()> {
    unsafe {
        let module = GetModuleHandleW(None)?;
        let instance = HINSTANCE(module.0);
        let class_name = w!("NanoInstallerNativeRuntime");
        let window_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassExW(&window_class) == 0 {
            return Err(windows::core::Error::from_win32().into());
        }
        let style = WS_POPUP;
        let left = (GetSystemMetrics(SM_CXSCREEN) - client_width) / 2;
        let top = (GetSystemMetrics(SM_CYSCREEN) - client_height) / 2;
        let window = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name,
            w!("nano-installer native Win32"),
            style,
            left,
            top,
            client_width,
            client_height,
            None,
            None,
            instance,
            None,
        )?;
        if let Some(ui) = UI.get() {
            if ui.corner_radius > 0 {
                let region = CreateRoundRectRgn(
                    0,
                    0,
                    client_width + 1,
                    client_height + 1,
                    ui.corner_radius * 2,
                    ui.corner_radius * 2,
                );
                if !region.is_invalid() {
                    let _ = SetWindowRgn(window, region, true);
                }
            }
        }
        let _ = ShowWindow(window, SW_SHOW);
        let _ = UpdateWindow(window);
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(window, &mut paint);
            if let Some(ui) = UI.get() {
                let mut client = RECT::default();
                let _ = GetClientRect(window, &mut client);
                for layer in &ui.layers {
                    draw_layer(dc, layer);
                }
                for text in &ui.texts {
                    draw_text(dc, text);
                }
            }
            let _ = EndPaint(window, &paint);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = client_point(lparam);
            if let Some(region) = UI.get().and_then(|ui| {
                ui.actions.iter().find(|region| {
                    x >= region.left && x < region.right && y >= region.top && y < region.bottom
                })
            }) {
                match region.action {
                    WindowAction::Close => {
                        let _ = DestroyWindow(window);
                    }
                    WindowAction::Minimize => {
                        let _ = ShowWindow(window, SW_MINIMIZE);
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = client_point(lparam);
            let over_action = UI.get().is_some_and(|ui| {
                ui.actions.iter().any(|region| {
                    x >= region.left && x < region.right && y >= region.top && y < region.bottom
                })
            });
            if y < 64 && !over_action {
                let _ = ReleaseCapture();
                let _ = SendMessageW(window, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), lparam);
            }
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u32 == 0x1B => {
            let _ = DestroyWindow(window);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

fn client_point(lparam: LPARAM) -> (i32, i32) {
    let x = lparam.0 as i16 as i32;
    let y = (lparam.0 >> 16) as i16 as i32;
    (x, y)
}

unsafe fn draw_text(destination: HDC, layer: &TextLayer) {
    let font = CreateFontW(
        -layer.font_size,
        0,
        0,
        0,
        if layer.bold {
            FW_BOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        w!("Microsoft YaHei"),
    );
    if font.is_invalid() {
        return;
    }
    let previous = SelectObject(destination, font);
    let _ = SetBkMode(destination, TRANSPARENT);
    let _ = SetTextColor(destination, layer.color);
    let mut bounds = RECT {
        left: layer.left,
        top: layer.top,
        right: layer.left + layer.width,
        bottom: layer.top + layer.height,
    };
    let mut text: Vec<u16> = layer.text.encode_utf16().collect();
    let alignment = if layer.centered { DT_CENTER } else { DT_LEFT };
    let _ = DrawTextW(
        destination,
        &mut text,
        &mut bounds,
        alignment | DT_VCENTER | DT_SINGLELINE,
    );
    let _ = SelectObject(destination, previous);
    let _ = DeleteObject(font);
}

unsafe fn draw_layer(destination: HDC, layer: &ImageLayer) {
    let source = CreateCompatibleDC(destination);
    if source.is_invalid() {
        return;
    }
    let mut bitmap_info = BITMAPINFO::default();
    bitmap_info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: layer.image.width as i32,
        biHeight: -(layer.image.height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let Ok(bitmap) = CreateDIBSection(
        destination,
        &bitmap_info,
        DIB_RGB_COLORS,
        &mut bits,
        None,
        0,
    ) else {
        let _ = DeleteDC(source);
        return;
    };
    std::ptr::copy_nonoverlapping(
        layer.image.pixels.as_ptr(),
        bits.cast(),
        layer.image.pixels.len(),
    );
    let previous = SelectObject(source, bitmap);
    let _ = GdiAlphaBlend(
        destination,
        layer.left,
        layer.top,
        layer.width,
        layer.height,
        source,
        0,
        0,
        layer.image.width as i32,
        layer.image.height as i32,
        BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        },
    );
    let _ = SelectObject(source, previous);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(source);
}

#[cfg(test)]
mod tests {
    use super::{image_path_from_style, pack_project, parse_bundle};

    #[test]
    fn project_bundle_roundtrips_layout_assets_and_locales() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path();
        for directory in ["layouts", "assets", "locales"] {
            std::fs::create_dir(project.join(directory))?;
        }
        std::fs::write(
            project.join("installer_config.json"),
            br#"{"resources":{"payload_file":"payload/app.7z"},"wizard":{"pages":[{"layout":"layouts/config.xml"}]}}"#,
        )?;
        std::fs::write(
            project.join("layouts/config.xml"),
            br#"<Page width="720" height="450" />"#,
        )?;
        std::fs::write(project.join("assets/background.png"), b"png")?;
        std::fs::write(project.join("locales/zh-CN.json"), b"{}")?;
        std::fs::create_dir(project.join("payload"))?;
        std::fs::write(project.join("payload/app.7z"), b"payload")?;

        let packed = pack_project(project)?;
        let files = parse_bundle(&packed)?;

        assert_eq!(files.len(), 5);
        assert!(files.contains_key("installer_config.json"));
        assert!(files.contains_key("layouts/config.xml"));
        assert!(files.contains_key("assets/background.png"));
        assert!(files.contains_key("locales/zh-CN.json"));
        assert_eq!(files.get("payload/app.7z").unwrap(), b"payload");
        Ok(())
    }

    #[test]
    fn image_style_supports_plain_and_nsis_paths() {
        assert_eq!(
            image_path_from_style("assets/button.png"),
            Some("assets/button.png")
        );
        assert_eq!(
            image_path_from_style("file='assets/button.png' dest='0,0,10,10'"),
            Some("assets/button.png")
        );
    }
}
