#![windows_subsystem = "windows"] // Prevents a console window from appearing

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::sync::{Arc, Mutex};
use std::ptr::null;
use std::mem::{size_of, zeroed};
use std::thread;

// Import necessary items from the windows crate
use windows::{
    core::*,
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW,
        // Added VK codes for arrows, plus, minus
        UI::Input::KeyboardAndMouse::{*, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_OEM_PLUS, VK_OEM_MINUS},
        // Added SetWindowPos flags
        UI::WindowsAndMessaging::{*, SWP_NOSIZE, SWP_NOMOVE, SWP_NOZORDER, SWP_NOACTIVATE},
    },
};

// --- API/Encoding Imports ---
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use image::{ImageBuffer, Rgba, ImageFormat};
use reqwest::blocking::Client;
use serde::Serialize;
use std::io::Cursor;
// --- End API/Encoding Imports ---


// Global variable to hold the window handle (HWND)
static WINDOW_HANDLE: once_cell::sync::Lazy<Arc<Mutex<Option<HWND>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

// Global variable to hold the captured image data (pixels, width, height)
static CAPTURED_IMAGE_DATA: once_cell::sync::Lazy<Arc<Mutex<Option<(Vec<u8>, i32, i32)>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

// Global variable to hold the response text from the API
static API_RESPONSE_TEXT: once_cell::sync::Lazy<Arc<Mutex<Option<String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

// Global variable to track if an API request is in progress
static IS_SENDING: once_cell::sync::Lazy<Arc<Mutex<bool>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(false)));


// Define unique IDs for our hotkeys
const HOTKEY_ID_TOGGLE: i32 = 1; // Ctrl+B
const HOTKEY_ID_CAPTURE: i32 = 2; // Ctrl+H
const HOTKEY_ID_SEND: i32 = 3; // Ctrl+Enter
const HOTKEY_ID_START_OVER: i32 = 4; // Ctrl+R
// New Hotkey IDs
const HOTKEY_ID_MOVE_LEFT: i32 = 5; // Ctrl+Left
const HOTKEY_ID_MOVE_RIGHT: i32 = 6; // Ctrl+Right
const HOTKEY_ID_MOVE_UP: i32 = 7; // Ctrl+Up
const HOTKEY_ID_MOVE_DOWN: i32 = 8; // Ctrl+Down
const HOTKEY_ID_RESIZE_LARGER: i32 = 9; // Ctrl+Plus
const HOTKEY_ID_RESIZE_SMALLER: i32 = 10; // Ctrl+Minus


// Placeholder for your proxy server URL - REPLACE THIS!
const PROXY_SERVER_URL: &str = "http://192.168.1.239:3000/process-image"; // Corrected URL with path

// Struct for the JSON payload to send to the proxy
#[derive(Serialize)]
struct ApiRequestPayload<'a> {
    image_base64: &'a str,
    prompt: &'a str,
}


fn main() -> Result<()> {
    // --- Window Setup (Same as before) ---
    let instance: HINSTANCE = unsafe { GetModuleHandleW(PCWSTR(null()))?.into() };
    let class_name_wide: Vec<u16> = OsStr::new("SampleWindowClass").encode_wide().chain(once(0)).collect();
    let class_name_pcwstr = PCWSTR(class_name_wide.as_ptr());
    let wc = unsafe {
        WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC, lpfnWndProc: Some(window_proc),
            hInstance: instance, hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: class_name_pcwstr, ..Default::default()
        }
    };
    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 { panic!("Could not register window class, error code: {}", unsafe { GetLastError() }.0); }
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            class_name_pcwstr, w!("Vibe Coder"), WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, 600, 400, None, None, instance, None,
        )
    };
    if hwnd.0 == 0 { panic!("Could not create window, error code: {}", unsafe { GetLastError() }.0); }
    { let mut hwnd_guard = WINDOW_HANDLE.lock().unwrap(); *hwnd_guard = Some(hwnd); }
    unsafe { if SetLayeredWindowAttributes(hwnd, COLORREF(0), 180u8, LWA_ALPHA).is_err() { println!("Warning: Could not set layered window attributes"); } }
    unsafe { if SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE).is_err() { println!("Warning: Could not set window display affinity"); } else { println!("Window set to be excluded from capture."); } }
    // --- End Window Setup ---


    // --- Register Global Hotkeys ---
    unsafe {
        let modifier = MOD_CONTROL; // Use Ctrl for all new hotkeys
        // Existing Hotkeys
        if RegisterHotKey(hwnd, HOTKEY_ID_TOGGLE, modifier, VK_B.0 as u32).is_err() { panic!("Could not register hotkey (Ctrl+B)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_CAPTURE, modifier, VK_H.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+H)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_SEND, modifier, VK_RETURN.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+Enter)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_START_OVER, modifier, VK_R.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+R)"); }
        // New Movement Hotkeys
        if RegisterHotKey(hwnd, HOTKEY_ID_MOVE_LEFT, modifier, VK_LEFT.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+Left)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_MOVE_RIGHT, modifier, VK_RIGHT.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+Right)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_MOVE_UP, modifier, VK_UP.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+Up)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_MOVE_DOWN, modifier, VK_DOWN.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+Down)"); }
        // New Resize Hotkeys (VK_OEM_PLUS is usually '=', VK_OEM_MINUS is '-')
        if RegisterHotKey(hwnd, HOTKEY_ID_RESIZE_LARGER, modifier, VK_OEM_PLUS.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl++)"); }
        if RegisterHotKey(hwnd, HOTKEY_ID_RESIZE_SMALLER, modifier, VK_OEM_MINUS.0 as u32).is_err() { println!("Warning: Could not register hotkey (Ctrl+-)"); }

        println!("Hotkeys registered: Ctrl+B (Toggle), Ctrl+H (Capture), Ctrl+Enter (Send), Ctrl+R (Reset), Ctrl+Arrows (Move), Ctrl +/- (Resize)");
    }
    // --- End hotkey registration ---

    // --- HTTP Client Setup ---
    let http_client = Arc::new(Client::new());
    // --- End HTTP Client Setup ---


    // Standard Windows message loop
    let mut message = MSG::default();
    unsafe {
        while bool::from(GetMessageW(&mut message, HWND(0), 0, 0)) {
            if message.message == WM_HOTKEY {
                match message.wParam.0 as i32 {
                    HOTKEY_ID_TOGGLE => {
                        println!("Hotkey Ctrl+B pressed!");
                        toggle_window_visibility(hwnd);
                    }
                    HOTKEY_ID_CAPTURE => {
                        println!("Hotkey Ctrl+H pressed!");
                        if *IS_SENDING.lock().unwrap() {
                             println!("Already sending, cannot capture now.");
                             continue;
                        }
                        match capture_screen_to_memory() {
                            Ok(_) => {
                                println!("Screen captured successfully to memory.");
                                { API_RESPONSE_TEXT.lock().unwrap().take(); }
                                let _ = InvalidateRect(hwnd, None, true);
                            },
                            Err(e) => println!("Screen capture failed: {}", e),
                        }
                    }
                    HOTKEY_ID_SEND => {
                        println!("Hotkey Ctrl+Enter pressed!");
                        if *IS_SENDING.lock().unwrap() {
                            println!("Already sending request...");
                        } else if CAPTURED_IMAGE_DATA.lock().unwrap().is_none() {
                            println!("No image captured yet. Press Ctrl+H first.");
                        } else {
                             let client_clone = Arc::clone(&http_client);
                             let window_handle_clone = Arc::clone(&WINDOW_HANDLE);
                             thread::spawn(move || {
                                 let hwnd_for_thread = match *window_handle_clone.lock().unwrap() { Some(h) => h, None => { println!("Error: Window handle not available in sending thread."); return; } };
                                 { *IS_SENDING.lock().unwrap() = true; }
                                 #[allow(unused_unsafe)] unsafe { let _ = InvalidateRect(hwnd_for_thread, None, true); }
                                 send_captured_image_task(&client_clone);
                                 { *IS_SENDING.lock().unwrap() = false; }
                                 #[allow(unused_unsafe)] unsafe { let _ = InvalidateRect(hwnd_for_thread, None, true); }
                             });
                        }
                    }
                    HOTKEY_ID_START_OVER => {
                        println!("Hotkey Ctrl+R pressed! Starting over.");
                        { CAPTURED_IMAGE_DATA.lock().unwrap().take(); }
                        { API_RESPONSE_TEXT.lock().unwrap().take(); }
                        { *IS_SENDING.lock().unwrap() = false; }
                        let _ = InvalidateRect(hwnd, None, true);
                    }
                    // --- Handle New Hotkeys ---
                    HOTKEY_ID_MOVE_LEFT => move_window(hwnd, -10, 0), // Move left by 10 pixels
                    HOTKEY_ID_MOVE_RIGHT => move_window(hwnd, 10, 0), // Move right by 10 pixels
                    HOTKEY_ID_MOVE_UP => move_window(hwnd, 0, -10), // Move up by 10 pixels
                    HOTKEY_ID_MOVE_DOWN => move_window(hwnd, 0, 10), // Move down by 10 pixels
                    HOTKEY_ID_RESIZE_LARGER => resize_window(hwnd, 20, 20), // Increase size by 20x20
                    HOTKEY_ID_RESIZE_SMALLER => resize_window(hwnd, -20, -20), // Decrease size by 20x20
                    // --- End Handle New Hotkeys ---
                    _ => {}
                }
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    // Unregister Hotkeys on exit
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_TOGGLE);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_CAPTURE);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_SEND);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_START_OVER);
        // Unregister new hotkeys
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_LEFT);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_RIGHT);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_UP);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_DOWN);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_RESIZE_LARGER);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_RESIZE_SMALLER);
        println!("Hotkeys unregistered.");
    }

    Ok(())
}

// Function to toggle window visibility - unchanged
fn toggle_window_visibility(hwnd: HWND) {
    unsafe {
        let is_visible = IsWindowVisible(hwnd).as_bool();
        if is_visible {
            println!("Hiding window...");
            let _ = ShowWindow(hwnd, SW_HIDE);
        } else {
            println!("Showing window...");
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = InvalidateRect(hwnd, None, true);
        }
    }
}

// Screen Capture Function (Stores pixel data in memory) - unchanged
fn capture_screen_to_memory() -> Result<()> {
    println!("Attempting GDI screen capture to memory...");
    #[allow(unused_assignments)]
    let mut hbm_capture_option: Option<HBITMAP> = None;
    let result = unsafe {
        let hdc_screen = GetDC(HWND(0)); if hdc_screen.is_invalid() { return Err(Error::from_win32()); }
        let hdc_mem = CreateCompatibleDC(hdc_screen); if hdc_mem.is_invalid() { let _ = ReleaseDC(HWND(0), hdc_screen); return Err(Error::from_win32()); }
        let screen_width = GetDeviceCaps(hdc_screen, HORZRES); let screen_height = GetDeviceCaps(hdc_screen, VERTRES);
        let hbm = CreateCompatibleBitmap(hdc_screen, screen_width, screen_height); if hbm.is_invalid() { let _ = DeleteDC(hdc_mem); let _ = ReleaseDC(HWND(0), hdc_screen); return Err(Error::from_win32()); }
        hbm_capture_option = Some(hbm); let hbm_capture = hbm;
        let hbm_old = SelectObject(hdc_mem, hbm_capture); if hbm_old.is_invalid() { let _ = DeleteDC(hdc_mem); let _ = ReleaseDC(HWND(0), hdc_screen); return Err(Error::from_win32()); }
        let bitblt_result = BitBlt(hdc_mem, 0, 0, screen_width, screen_height, hdc_screen, 0, 0, SRCCOPY);
        let _ = SelectObject(hdc_mem, hbm_old); let _ = DeleteDC(hdc_mem); let _ = ReleaseDC(HWND(0), hdc_screen);
        if bitblt_result.is_err() { println!("BitBlt failed."); return bitblt_result; }
        let mut bmih: BITMAPINFOHEADER = zeroed(); bmih.biSize = size_of::<BITMAPINFOHEADER>() as u32; bmih.biWidth = screen_width; bmih.biHeight = -screen_height; bmih.biPlanes = 1; bmih.biBitCount = 32; bmih.biCompression = BI_RGB.0 as u32;
        let image_size = (screen_width * screen_height * 4) as usize; let mut pixel_data: Vec<u8> = vec![0; image_size];
        let hdc_temp = GetDC(HWND(0)); if hdc_temp.is_invalid() { return Err(Error::from_win32()); }
        let lines = GetDIBits(hdc_temp, hbm_capture, 0, screen_height as u32, Some(pixel_data.as_mut_ptr() as *mut _), &mut bmih as *mut _ as *mut BITMAPINFO, DIB_RGB_COLORS);
        let _ = ReleaseDC(HWND(0), hdc_temp); if lines == 0 { println!("GetDIBits failed."); return Err(Error::from_win32()); }
        { let mut image_data_guard = CAPTURED_IMAGE_DATA.lock().unwrap(); println!("Storing image data globally ({} bytes)", pixel_data.len()); *image_data_guard = Some((pixel_data, screen_width, screen_height)); }
        Ok(())
    };
    if let Some(handle) = hbm_capture_option { unsafe { let _ = DeleteObject(handle); } }
    result
}

// Background Task for Encoding and Sending Image - unchanged
fn send_captured_image_task(client: &Client) {
    println!("Background task: Preparing to send image...");
    let maybe_image_data = CAPTURED_IMAGE_DATA.lock().unwrap().clone();
    if let Some((pixel_data, width, height)) = maybe_image_data {
        println!("Background task: Image data found ({}x{}). Encoding...", width, height);
        let mut rgba_pixel_data = pixel_data.clone();
        for chunk in rgba_pixel_data.chunks_exact_mut(4) { chunk.swap(0, 2); }
        let img_buffer = match ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width as u32, height as u32, rgba_pixel_data) { Some(buffer) => buffer, None => { println!("Error: Could not create image buffer."); return; } };
        let mut png_buffer = Cursor::new(Vec::new());
        if let Err(e) = img_buffer.write_to(&mut png_buffer, ImageFormat::Png) { println!("Error encoding image to PNG: {}", e); { *API_RESPONSE_TEXT.lock().unwrap() = Some(format!("PNG Encoding Error: {}", e)); } return; }
        let base64_image = BASE64_STANDARD.encode(png_buffer.get_ref());
        println!("Background task: Image encoded to Base64 ({} bytes)", base64_image.len());
        let payload = ApiRequestPayload { image_base64: &base64_image, prompt: "Analyze the coding problem shown in the image. Provide the optimal code solution with comments explaining the logic and include the time complexity analysis. Format the code solution within a markdown code block (e.g., ```python ... ```).", };
        println!("Background task: Sending request to proxy: {}", PROXY_SERVER_URL);
        let response_result = client.post(PROXY_SERVER_URL).json(&payload).send();
        match response_result {
            Ok(response) => {
                println!("Background task: Proxy response status: {}", response.status());
                match response.text() {
                    Ok(text) => { println!("Background task: Received text from proxy: {}...", text.chars().take(100).collect::<String>()); { *API_RESPONSE_TEXT.lock().unwrap() = Some(text); } { CAPTURED_IMAGE_DATA.lock().unwrap().take(); } },
                    Err(e) => { println!("Error reading text from proxy response: {}", e); { *API_RESPONSE_TEXT.lock().unwrap() = Some(format!("Proxy Response Error: {}", e)); } }
                }
            },
            Err(e) => { println!("Error sending request to proxy: {}", e); { *API_RESPONSE_TEXT.lock().unwrap() = Some(format!("Proxy Send Error: {}", e)); } }
        }
    } else { println!("Background task Error: No captured image data found."); { *API_RESPONSE_TEXT.lock().unwrap() = Some("Error: No image data was available to send.".to_string()); } }
}

// --- Function to Move Window ---
fn move_window(hwnd: HWND, dx: i32, dy: i32) {
    unsafe {
        let mut rect = RECT::default();
        // Check GetWindowRect result using is_ok()
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let new_x = rect.left + dx;
            let new_y = rect.top + dy;
            let flags = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
            // Check SetWindowPos result using is_err()
            if SetWindowPos(hwnd, HWND_TOPMOST, new_x, new_y, 0, 0, flags).is_err() {
                 println!("SetWindowPos (move) failed: {:?}", GetLastError());
            } else {
                println!("Moved window to ({}, {})", new_x, new_y);
            }
        } else {
            println!("GetWindowRect failed in move_window: {:?}", GetLastError());
        }
    }
}

// --- Function to Resize Window ---
fn resize_window(hwnd: HWND, dw: i32, dh: i32) {
    unsafe {
        let mut rect = RECT::default();
         // Check GetWindowRect result using is_ok()
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let current_width = rect.right - rect.left;
            let current_height = rect.bottom - rect.top;
            let new_width = (current_width + dw).max(100); // Min size 100x100
            let new_height = (current_height + dh).max(100);

            let flags = SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE;
             // Check SetWindowPos result using is_err()
            if SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, new_width, new_height, flags).is_err() {
                 println!("SetWindowPos (resize) failed: {:?}", GetLastError());
            } else {
                println!("Resized window to {}x{}", new_width, new_height);
                let _ = InvalidateRect(hwnd, None, true); // Force redraw after resize
            }
        } else {
            println!("GetWindowRect failed in resize_window: {:?}", GetLastError());
        }
    }
}


// Helper function to draw text (contains unsafe calls) - unchanged
unsafe fn draw_window_text(hdc: HDC, rect: &mut RECT, text: &str) -> Result<()> {
    let mut wide_text: Vec<u16> = text.encode_utf16().collect();
    SetTextColor(hdc, COLORREF(0x00FFFFFF)); // White text
    SetBkMode(hdc, TRANSPARENT);
    let result = DrawTextW(
        hdc, &mut wide_text, rect as *mut _,
        DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX | DT_EDITCONTROL,
    );
    if result == 0 { Err(Error::from_win32()) } else { Ok(()) }
}

// Helper function to draw bitmap as a preview (contains unsafe calls) - unchanged
unsafe fn draw_captured_bitmap_preview(hdc_window: HDC, client_rect: &RECT, image_data: &[u8], img_width: i32, img_height: i32) -> Result<()> {
    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = img_width; bmi.bmiHeader.biHeight = -img_height;
    bmi.bmiHeader.biPlanes = 1; bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0 as u32;
    let client_width = client_rect.right - client_rect.left; let client_height = client_rect.bottom - client_rect.top;
    let padding = 10; let max_preview_width = (client_width - 2 * padding).max(1); let max_preview_height = (client_height - 2 * padding).max(1);
    let aspect_ratio = img_width as f32 / img_height as f32; let mut preview_width = max_preview_width; let mut preview_height = (preview_width as f32 / aspect_ratio) as i32;
    if preview_height > max_preview_height { preview_height = max_preview_height; preview_width = (preview_height as f32 * aspect_ratio) as i32; }
    let preview_x = padding + (max_preview_width - preview_width) / 2; let preview_y = padding + (max_preview_height - preview_height) / 2;
    let result = StretchDIBits(
        hdc_window, preview_x, preview_y, preview_width, preview_height, 0, 0, img_width, img_height,
        Some(image_data.as_ptr() as *const _), &bmi, DIB_RGB_COLORS, SRCCOPY,
    );
    if result == GDI_ERROR as i32 || result == 0 { println!("StretchDIBits failed."); Err(Error::from_win32()) } else { Ok(()) }
}


// Window Procedure function
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if hdc.is_invalid() { return LRESULT(0); }

            let mut client_rect: RECT = zeroed();
            if GetClientRect(hwnd, &mut client_rect).is_err() {
                 println!("GetClientRect failed in WM_PAINT");
                 let _ = EndPaint(hwnd, &ps); return LRESULT(0);
            }

            let is_sending_guard = IS_SENDING.lock().unwrap();
            let image_data_guard = CAPTURED_IMAGE_DATA.lock().unwrap();
            let response_text_guard = API_RESPONSE_TEXT.lock().unwrap();

            // --- Determine what to draw ---
            let draw_result = if *is_sending_guard {
                println!("Drawing Sending... text");
                let sending_text = "Sending request to AI...";
                draw_window_text(hdc, &mut client_rect, sending_text)
            }
            else if let Some(text) = &*response_text_guard {
                 println!("Drawing API response text...");
                 let mut text_rect = client_rect;
                 text_rect.left += 10; text_rect.right -= 10;
                 text_rect.top += 10; text_rect.bottom -= 10;
                 draw_window_text(hdc, &mut text_rect, text)
            } else if let Some((pixels, width, height)) = &*image_data_guard {
                 println!("Drawing captured bitmap preview...");
                 draw_captured_bitmap_preview(hdc, &client_rect, pixels, *width, *height)
            } else {
                 println!("Drawing default text...");
                 // Updated welcome text
                 let welcome_text = "Welcome to Vibe Coder!\n\nCtrl+H: Capture\nCtrl+B: Toggle\nCtrl+Enter: Send\nCtrl+R: Reset\nCtrl+Arrows: Move\nCtrl +/-: Resize";
                 draw_window_text(hdc, &mut client_rect, welcome_text)
            };
            // --- End drawing logic ---

            if draw_result.is_err() {
                println!("Drawing failed in WM_PAINT");
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            println!("WM_DESTROY received");
            { CAPTURED_IMAGE_DATA.lock().unwrap().take(); }
            { API_RESPONSE_TEXT.lock().unwrap().take(); }
            { *IS_SENDING.lock().unwrap() = false; }
            unsafe {
                // Unregister all hotkeys
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_TOGGLE);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_CAPTURE);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_SEND);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_START_OVER);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_LEFT);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_RIGHT);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_UP);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_MOVE_DOWN);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_RESIZE_LARGER);
                let _ = UnregisterHotKey(hwnd, HOTKEY_ID_RESIZE_SMALLER);
                println!("Hotkeys unregistered in window_proc.");
            }
            unsafe { PostQuitMessage(0); }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
