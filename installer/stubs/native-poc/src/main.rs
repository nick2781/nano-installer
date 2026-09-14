#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "x86_64"))]
compile_error!("native-ui-x64 must be built for x86_64");

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, PAINTSTRUCT, SRCCOPY,
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
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClientRect, GetMessageW, LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassExW,
    ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW,
    MB_ICONERROR, MB_OK, MSG, SW_SHOW, WINDOW_EX_STYLE, WM_CREATE, WM_DESTROY, WM_KEYDOWN,
    WM_PAINT, WNDCLASSEXW, WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

static IMAGE: OnceLock<NativeImage> = OnceLock::new();

struct NativeImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn main() {
    if let Err(error) = run() {
        let message = HSTRING::from(format!("Native runtime POC failed:\n{error:#}"));
        unsafe {
            MessageBoxW(None, &message, w!("nano-installer"), MB_OK | MB_ICONERROR);
        }
    }
}

fn run() -> Result<()> {
    let image_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/TapTap/assets/bg_main.png"));
    let image = decode_png(&image_path)?;
    let client_width = image.width as i32;
    let client_height = image.height as i32;
    IMAGE
        .set(image)
        .map_err(|_| anyhow::anyhow!("native image was already initialized"))?;

    unsafe {
        let module = GetModuleHandleW(None)?;
        let instance = HINSTANCE(module.0);
        let class_name = w!("NanoInstallerNativePoc");
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

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut bounds = RECT {
            left: 0,
            top: 0,
            right: client_width,
            bottom: client_height,
        };
        AdjustWindowRectEx(&mut bounds, style, false, WINDOW_EX_STYLE::default())?;

        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("nano-installer native Win32 POC"),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            None,
            None,
            instance,
            None,
        )?;
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

fn decode_png(path: &Path) -> Result<NativeImage> {
    let encoded = std::fs::read(path)
        .with_context(|| format!("failed to read bitmap source: {}", path.display()))?;

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let _guard = ComGuard;
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = factory.CreateStream()?;
        stream.InitializeFromMemory(&encoded)?;
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
        let buffer_len = stride
            .checked_mul(height)
            .context("bitmap buffer size overflow")? as usize;
        let mut pixels = vec![0u8; buffer_len];
        converter.CopyPixels(std::ptr::null(), stride, &mut pixels)?;

        Ok(NativeImage {
            width,
            height,
            pixels,
        })
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let _ = lparam.0 as *const CREATESTRUCTW;
            LRESULT(0)
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(window, &mut paint);
            if let Some(image) = IMAGE.get() {
                let mut client = RECT::default();
                let _ = GetClientRect(window, &mut client);
                let mut bitmap_info = BITMAPINFO::default();
                bitmap_info.bmiHeader = BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: image.width as i32,
                    biHeight: -(image.height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                };
                StretchDIBits(
                    dc,
                    0,
                    0,
                    client.right - client.left,
                    client.bottom - client.top,
                    0,
                    0,
                    image.width as i32,
                    image.height as i32,
                    Some(image.pixels.as_ptr().cast()),
                    &bitmap_info,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );
            }
            let _ = EndPaint(window, &paint);
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
