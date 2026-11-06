//! Pet Bolha - Windows Overlay
//!
//! Janela transparente, click-through, always-on-top, mínima CPU.
//! Usa tiny-skia para renderização CPU (sem GPU, sem Vulkan).
//!
//! ## Flags de janela (Win10+):
//! - WS_EX_TRANSPARENT: click-through cross-process
//! - WS_EX_NOACTIVATE: nunca rouba foco
//! - WS_EX_TOOLWINDOW: não aparece no Alt+Tab/barra
//! - HWND_TOPMOST: sempre por cima
//! - SEM WS_EX_LAYERED: evita custo DWM redirection surface

use std::ffi::c_void;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use pet_core::ffi::*;
use pet_tts::{PiperConfig, TtsEngine};
use tiny_skia::{Pixmap, Paint, PathBuilder, Transform, Color, PixmapPaint, PixmapRef};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_CONTROL, MOD_ALT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS};
use windows::core::PCWSTR;
use std::ffi::CString;
use tokio::sync::Mutex as TokioMutex;
use chrono;

const CLASS_NAME: &str = "PetBolhaOverlay";
const TARGET_FPS: u32 = 5;
const IDLE_RENDER_INTERVAL: u64 = 2000;
const PANIC_KEY: i32 = 0x50;
const PANIC_MODS: u32 = MOD_CONTROL.0 | MOD_ALT.0 | MOD_SHIFT.0;

struct OverlayState {
    hwnd: HWND,
    pet_handle: *mut PetHandle,
    running: Arc<AtomicBool>,
    awake: Arc<AtomicBool>,
    last_render: Mutex<Instant>,
    cursor_pos: Mutex<(i32, i32)>,
    virtual_screen: (i32, i32, i32, i32),
    sprite_frames: Vec<Pixmap>,  // frames pré-extraídos 64x64
    current_frame: usize,
    animation_timer: Instant,
    speech_bubble: Mutex<Option<String>>,
    speech_timer: Mutex<Instant>,
    tts_engine: Arc<TokioMutex<Option<TtsEngine>>>,  // TTS opcional
}

impl OverlayState {
    fn new(pet_handle: *mut PetHandle) -> anyhow::Result<Self> {
        let virtual_screen = unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            (x, y, w, h)
        };
        
        // Tenta inicializar TTS (não falha se não tiver)
        let tts_engine = Arc::new(TokioMutex::new(
            TtsEngine::new(PiperConfig::default()).ok()
        ));
        
        Ok(Self {
            hwnd: HWND(std::ptr::null_mut()),
            pet_handle,
            running: Arc::new(AtomicBool::new(true)),
            awake: Arc::new(AtomicBool::new(true)),
            last_render: Mutex::new(Instant::now()),
            cursor_pos: Mutex::new((0, 0)),
            virtual_screen,
            sprite_frames: Vec::new(),
            current_frame: 0,
            animation_timer: Instant::now(),
            speech_bubble: Mutex::new(None),
            speech_timer: Mutex::new(Instant::now()),
            tts_engine,
        })
    }

    fn register_class() -> anyhow::Result<()> {
        let class_name_w: Vec<u16> = CLASS_NAME.encode_utf16().chain(Some(0)).collect();
        
        let wc = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: unsafe { GetModuleHandleW(None)?.into() },
            hIcon: HICON(std::ptr::null_mut()),
            hCursor: HCURSOR(std::ptr::null_mut()),
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszMenuName: PCWSTR(ptr::null()),
            lpszClassName: PCWSTR(class_name_w.as_ptr()),
            hIconSm: HICON(std::ptr::null_mut()),
        };
        
        unsafe {
            if RegisterClassExW(&wc) == 0 {
                anyhow::bail!("RegisterClassExW failed");
            }
        }
        Ok(())
    }

    fn create_window(&mut self) -> anyhow::Result<()> {
        let class_name_w: Vec<u16> = CLASS_NAME.encode_utf16().chain(Some(0)).collect();
        let title_w: Vec<u16> = "Pet Bolha".encode_utf16().chain(Some(0)).collect();
        
        let (vx, vy, vw, vh) = self.virtual_screen;
        
        let ex_style = WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0;
        let style = WS_POPUP.0;
        
        self.hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(ex_style),
                PCWSTR(class_name_w.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                WINDOW_STYLE(style),
                vx, vy, vw, vh,
                None, None,
                GetModuleHandleW(None)?,
                Some(self as *mut _ as *mut c_void),
            )?
        };
        
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        
        unsafe {
            RegisterHotKey(self.hwnd, 1, HOT_KEY_MODIFIERS(PANIC_MODS), PANIC_KEY as u32);
        }
        
        Ok(())
    }

    fn load_sprite_sheet(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        
        // Extrai frames 64x64
        let frame_w = 64u32;
        let frame_h = 64u32;
        let cols = w / frame_w;
        let rows = h / frame_h;
        
        let mut frames = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let src_x = col * frame_w;
                let src_y = row * frame_h;
                
                let mut frame = Pixmap::new(frame_w, frame_h).ok_or_else(|| anyhow::anyhow!("Failed to create frame pixmap"))?;
                
                for fy in 0..frame_h {
                    for fx in 0..frame_w {
                        let src_idx = ((src_y + fy) * w + (src_x + fx)) as usize * 4;
                        let dst_idx = (fy * frame_w + fx) as usize * 4;
                        frame.data_mut()[dst_idx] = rgba.as_raw()[src_idx];
                        frame.data_mut()[dst_idx + 1] = rgba.as_raw()[src_idx + 1];
                        frame.data_mut()[dst_idx + 2] = rgba.as_raw()[src_idx + 2];
                        frame.data_mut()[dst_idx + 3] = rgba.as_raw()[src_idx + 3];
                    }
                }
                
                frames.push(frame);
            }
        }
        
        self.sprite_frames = frames;
        Ok(())
    }

    fn render_frame(&mut self) {
            let should_render = {
                let awake = self.awake.load(Ordering::Relaxed);
                let pet_handle = self.pet_handle;
            
                if !awake {
                    false
                } else if pet_handle.is_null() {
                    false
                } else {
                    unsafe { pet_should_render(pet_handle) != 0 }
                }
            };
        
            if !should_render {
                return;
            }
        
            let now = Instant::now();
            let min_interval = if self.awake.load(Ordering::Relaxed) {
                Duration::from_millis(1000 / TARGET_FPS as u64)
            } else {
                Duration::from_millis(IDLE_RENDER_INTERVAL)
            };
        
            {
                let mut last = self.last_render.lock().unwrap();
                if now.duration_since(*last) < min_interval {
                    return;
                }
                *last = now;
            }
        
            let dt = min_interval.as_secs_f32();
            unsafe { pet_tick(self.pet_handle, dt); }
        
            let mut state_json_ptr = ptr::null_mut();
            unsafe { pet_get_state(self.pet_handle, &mut state_json_ptr); }
            let state_json = if state_json_ptr.is_null() {
                String::new()
            } else {
                let s = unsafe { std::ffi::CStr::from_ptr(state_json_ptr).to_string_lossy().into_owned() };
                unsafe { pet_free_string(state_json_ptr); }
                s
            };
        
            let mut anim_ptr = ptr::null_mut();
            unsafe { pet_get_animation(self.pet_handle, &mut anim_ptr); }
            let animation = if anim_ptr.is_null() {
                "idle".to_string()
            } else {
                let a = unsafe { std::ffi::CStr::from_ptr(anim_ptr).to_string_lossy().into_owned() };
                unsafe { pet_free_string(anim_ptr); }
                a
            };
        
            let mut speech_ptr = ptr::null_mut();
            if unsafe { pet_get_speech(self.pet_handle, &mut speech_ptr) } == 0 {
                if !speech_ptr.is_null() {
                    let text = unsafe { std::ffi::CStr::from_ptr(speech_ptr).to_string_lossy().into_owned() };
                    unsafe { pet_free_string(speech_ptr); }
                    *self.speech_bubble.lock().unwrap() = Some(text.clone());
                    *self.speech_timer.lock().unwrap() = Instant::now();
                    
                    // TTS: sintetiza fala em background
                    let tts_engine = self.tts_engine.clone();
                    std::thread::spawn(move || {
                        let mut engine_guard = tts_engine.blocking_lock();
                        if let Some(engine) = engine_guard.as_mut() {
                            let filename = format!("pet_speech_{}.wav", chrono::Utc::now().timestamp_millis());
                            if let Err(e) = engine.synthesize_to_file(&text, &filename) {
                                tracing::warn!("TTS failed: {}", e);
                            } else {
                                tracing::info!("TTS synthesized: {}", filename);
                                // TODO: tocar o áudio (rodar player ou integrar com áudio do Windows)
                            }
                        }
                    });
                }
            }
        
            {
                            let mut bubble = self.speech_bubble.lock().unwrap();
                            let timer = self.speech_timer.lock().unwrap();
                            if bubble.is_some() && timer.elapsed() > Duration::from_secs(5) {
                                *bubble = None;
                            }
                        }

                        let (pet_x, pet_y) = {
                            let cursor = self.cursor_pos.lock().unwrap();
                            if cursor.0 == 0 && cursor.1 == 0 {
                                let (vx, vy, vw, vh) = self.virtual_screen;
                                (vx + vw / 2 - 50, vy + vh / 2 - 50)
                            } else {
                                (cursor.0 - 50, cursor.1 - 50)
                            }
                        };

                        self.draw_frame(pet_x, pet_y, &animation, &state_json);
                    }
    
    fn draw_frame(&mut self, x: i32, y: i32, animation: &str, _state_json: &str) {
        let (_, _, vw, vh) = self.virtual_screen;
        let mut pixmap = match Pixmap::new(vw as u32, vh as u32) {
            Some(p) => p,
            None => return,
        };
        
        if !self.sprite_frames.is_empty() {
            self.draw_sprite(&mut pixmap, x, y, animation);
        } else {
            self.draw_fallback_pet(&mut pixmap, x, y);
        }
        
        if let Some(text) = self.speech_bubble.lock().unwrap().as_ref() {
            self.draw_speech_bubble(&mut pixmap, x + 32, y - 60, text);
        }
        
        self.present(&pixmap, x, y);
    }
    
    fn draw_sprite(&self, pixmap: &mut Pixmap, x: i32, y: i32, animation: &str) {
            if self.sprite_frames.is_empty() {
                return;
            }
        
            // Frame size 64x64, scale 1.5x
            let frame_w = 64.0 * 1.5;
            let frame_h = 64.0 * 1.5;
        
            let frame_idx = match animation {
                "idle" => (self.animation_timer.elapsed().as_millis() / 250) as usize % 4,
                "walk" => (self.animation_timer.elapsed().as_millis() / 125) as usize % 4,
                "sleep" => (self.animation_timer.elapsed().as_millis() / 500) as usize % 2,
                "happy" => (self.animation_timer.elapsed().as_millis() / 166) as usize % 3,
                "sad" => (self.animation_timer.elapsed().as_millis() / 333) as usize % 2,
                "angry" => (self.animation_timer.elapsed().as_millis() / 125) as usize % 3,
                "play" => (self.animation_timer.elapsed().as_millis() / 100) as usize % 4,
                "think" => (self.animation_timer.elapsed().as_millis() / 500) as usize % 2,
                _ => 0,
            };
        
            let frame = &self.sprite_frames[frame_idx % self.sprite_frames.len()];
        
            let mut paint = PixmapPaint::default();
        
            let dst_rect = tiny_skia::Rect::from_xywh(x as f32, y as f32, frame_w, frame_h).unwrap();
        
            let mut pixmap_mut = pixmap.as_mut();
            pixmap_mut.draw_pixmap(
                x, y,
                frame.as_ref(),
                &paint,
                Transform::identity(),
                None,
            );
        }
    
    fn draw_fallback_pet(&self, pixmap: &mut Pixmap, x: i32, y: i32) {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(100, 200, 255, 255));
        paint.anti_alias = true;
        
        let path = PathBuilder::from_circle(x as f32 + 48.0, y as f32 + 48.0, 48.0).unwrap();
        pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
    
    fn draw_speech_bubble(&self, pixmap: &mut Pixmap, x: i32, y: i32, text: &str) {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 255, 255, 240));
        paint.anti_alias = true;
        
        let char_w = 7.0;
        let padding = 12.0;
        let lines: Vec<&str> = text.lines().collect();
        let max_w = lines.iter().map(|l| l.len() as f32 * char_w).fold(0.0, f32::max);
        let bubble_w = max_w + padding * 2.0;
        let bubble_h = lines.len() as f32 * 20.0 + padding * 2.0;
        
        let bx = x as f32 - bubble_w / 2.0;
        let by = y as f32 - bubble_h - 10.0;
        
        let mut pb = PathBuilder::new();
        let r = 12.0;
        pb.move_to(bx + r, by);
        pb.line_to(bx + bubble_w - r, by);
        pb.quad_to(bx + bubble_w, by, bx + bubble_w, by + r);
        pb.line_to(bx + bubble_w, by + bubble_h - r);
        pb.quad_to(bx + bubble_w, by + bubble_h, bx + bubble_w - r, by + bubble_h);
        pb.line_to(bx + r, by + bubble_h);
        pb.quad_to(bx, by + bubble_h, bx, by + bubble_h - r);
        pb.line_to(bx, by + r);
        pb.quad_to(bx, by, bx + r, by);
        pb.close();
        let bubble_path = pb.finish().unwrap();
        
        pixmap.fill_path(&bubble_path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
    
    fn present(&self, pixmap: &Pixmap, _pet_x: i32, _pet_y: i32) {
        let mut bgra = vec![0u8; pixmap.width() as usize * pixmap.height() as usize * 4];
        for (i, pixel) in pixmap.pixels().iter().enumerate() {
            bgra[i * 4] = pixel.blue();
            bgra[i * 4 + 1] = pixel.green();
            bgra[i * 4 + 2] = pixel.red();
            bgra[i * 4 + 3] = pixel.alpha();
        }
        
        let mut bmi: BITMAPINFO = unsafe { mem::zeroed() };
        bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = pixmap.width() as i32;
        bmi.bmiHeader.biHeight = -(pixmap.height() as i32);
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        
        let hdc_screen = unsafe { GetDC(HWND(std::ptr::null_mut())) };
        let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };
        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let hbitmap = unsafe { CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) };
        
        let hbitmap = match hbitmap {
            Ok(h) => h,
            Err(_) => {
                unsafe {
                    DeleteDC(hdc_mem);
                    ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
                }
                return;
            }
        };
        
        let old_bitmap = unsafe { SelectObject(hdc_mem, hbitmap) };
        
        unsafe {
            std::ptr::copy_nonoverlapping(bgra.as_ptr() as *const c_void, bits_ptr, bgra.len());
        }
        
        let mut pt_src = POINT { x: 0, y: 0 };
        let mut pt_dst = POINT { x: self.virtual_screen.0, y: self.virtual_screen.1 };
        let mut size = SIZE { cx: pixmap.width() as i32, cy: pixmap.height() as i32 };
        let mut blend = BLENDFUNCTION {
            BlendOp: 0, // AC_SRC_OVER
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1, // AC_SRC_ALPHA
        };
        
        unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                hdc_screen,
                Some(&pt_dst),
                Some(&size),
                hdc_mem,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
        
        unsafe {
            SelectObject(hdc_mem, old_bitmap);
            DeleteObject(hbitmap);
            DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
        }
    }

    fn run_loop(&mut self) {
        let mut msg = MSG::default();
        
        while self.running.load(Ordering::Relaxed) {
            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() } {
                if msg.message == WM_QUIT {
                    self.running.store(false, Ordering::Relaxed);
                    break;
                }
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            
            self.render_frame();
            thread::sleep(Duration::from_millis(16));
        }
    }
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            let state = cs.lpCreateParams as *mut OverlayState;
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if state_ptr != 0 {
                let state = unsafe { &mut *(state_ptr as *mut OverlayState) };
                let x = (lparam.0 & 0xFFFF) as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i32;
                *state.cursor_pos.lock().unwrap() = (x, y);
                
                if !state.pet_handle.is_null() {
                    unsafe { pet_set_cursor(state.pet_handle, x as f32, y as f32); }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if state_ptr != 0 {
                let state = unsafe { &mut *(state_ptr as *mut OverlayState) };
                if !state.pet_handle.is_null() {
                    unsafe { pet_interact(state.pet_handle, 1); }
                }
            }
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if state_ptr != 0 {
                let state = unsafe { &mut *(state_ptr as *mut OverlayState) };
                if !state.pet_handle.is_null() {
                    unsafe { pet_interact(state.pet_handle, 0); }
                }
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            if wparam.0 == 1 {
                let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
                if state_ptr != 0 {
                    let state = unsafe { &mut *(state_ptr as *mut OverlayState) };
                    state.running.store(false, Ordering::Relaxed);
                    unsafe { PostQuitMessage(0); }
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0); }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    tracing::info!("Starting Pet Bolha overlay...");
    
    let name = CString::new("Bolha")?;
    let personality = CString::new("balanced")?;
    let config_json = CString::new(r#"{"speech_enabled":true,"neural_enabled":true}"#)?;
    
    let pet_handle = unsafe { 
        pet_create(name.as_ptr(), personality.as_ptr(), config_json.as_ptr())
    };
    
    if pet_handle.is_null() {
        anyhow::bail!("Failed to create pet");
    }
    
    unsafe { pet_load(pet_handle); }
    
    let mut overlay = OverlayState::new(pet_handle)?;
    OverlayState::register_class()?;
    overlay.create_window()?;
    
    let assets_dir = std::env::current_exe()?.parent().unwrap().join("assets").join("sprites");
    let sprite_path = assets_dir.join("bolha.png");
    if sprite_path.exists() {
        overlay.load_sprite_sheet(&sprite_path)?;
    }
    
    unsafe { ShowWindow(overlay.hwnd, SW_SHOWNOACTIVATE); }
    
    overlay.run_loop();
    
    unsafe { 
        pet_save(pet_handle);
        pet_destroy(pet_handle);
        UnregisterHotKey(overlay.hwnd, 1);
    }
    
    tracing::info!("Pet Bolha overlay stopped");
    Ok(())
}