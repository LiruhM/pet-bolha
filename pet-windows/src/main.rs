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
use tiny_skia::{Pixmap, Paint, PathBuilder, Transform, Color, Font, FontBuilder};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::{PCWSTR, HSTRING};

const CLASS_NAME: &str = "PetBolhaOverlay";
const TARGET_FPS: u32 = 5;           // Máximo 5 FPS quando acordado
const IDLE_RENDER_INTERVAL: u64 = 2000; // 1 frame a cada 2s quando idle (respiração)
const PANIC_KEY: i32 = 0x50;         // 'P' key
const PANIC_MODS: u32 = MOD_CONTROL | MOD_ALT | MOD_SHIFT;

struct OverlayState {
    hwnd: HWND,
    pet_handle: *mut PetHandle,
    running: Arc<AtomicBool>,
    awake: Arc<AtomicBool>,
    last_render: Mutex<Instant>,
    cursor_pos: Mutex<(i32, i32)>,
    virtual_screen: (i32, i32, i32, i32), // x, y, w, h
    sprite_sheet: Option<Pixmap>,
    current_frame: usize,
    animation_timer: Instant,
    speech_bubble: Mutex<Option<String>>,
    speech_timer: Mutex<Instant>,
}

impl OverlayState {
    fn new(pet_handle: *mut PetHandle) -> anyhow::Result<Self> {
        // Calcula tela virtual (todos monitores)
        let virtual_screen = unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            (x, y, w, h)
        };
        
        Ok(Self {
            hwnd: HWND(0),
            pet_handle,
            running: Arc::new(AtomicBool::new(true)),
            awake: Arc::new(AtomicBool::new(true)),
            last_render: Mutex::new(Instant::now()),
            cursor_pos: Mutex::new((0, 0)),
            virtual_screen,
            sprite_sheet: None,
            current_frame: 0,
            animation_timer: Instant::now(),
            speech_bubble: Mutex::new(None),
            speech_timer: Mutex::new(Instant::now()),
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
            hIcon: HICON(0),
            hCursor: HCURSOR(0),
            hbrBackground: HBRUSH(0),
            lpszMenuName: PCWSTR(ptr::null()),
            lpszClassName: PCWSTR(class_name_w.as_ptr()),
            hIconSm: HICON(0),
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
        
        // Win10+: WS_EX_TRANSPARENT sozinho já dá click-through
        // SEM WS_EX_LAYERED = sem custo DWM redirection surface
        let ex_style = WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0;
        let style = WS_POPUP.0;
        
        self.hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(ex_style),
                PCWSTR(class_name_w.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                WINDOW_STYLE(style),
                vx, vy, vw, vh,
                None, None,
                GetModuleHandleW(None)?.into(),
                Some(self as *mut _ as *mut c_void),
            )
        };
        
        if self.hwnd.0 == 0 {
            anyhow::bail!("CreateWindowExW failed");
        }
        
        // Always on top
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        
        // Registra hotkey de pânico (Ctrl+Alt+Shift+P)
        unsafe {
            RegisterHotKey(self.hwnd, 1, PANIC_MODS, PANIC_KEY);
        }
        
        Ok(())
    }

    fn load_sprite_sheet(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut pixmap = Pixmap::new(w, h).ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;
        pixmap.data.copy_from_slice(&rgba);
        self.sprite_sheet = Some(pixmap);
        Ok(())
    }

    fn render_frame(&mut self) {
        let should_render = {
            let awake = self.awake.load(Ordering::Relaxed);
            let pet_handle = self.pet_handle;
            
            if !awake {
                // Verifica se deve acordar (pet_tick dirá)
                false
            } else if pet_handle.is_null() {
                false
            } else {
                // Chama pet_should_render via FFI
                unsafe { pet_should_render(pet_handle) != 0 }
            }
        };
        
        if !should_render {
            return;
        }
        
        // Throttle FPS
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
        
        // Tick do pet (100ms step para neural)
        let dt = min_interval.as_secs_f32();
        unsafe { pet_tick(self.pet_handle, dt); }
        
        // Pega estado para render
        let mut state_json_ptr = ptr::null_mut();
        unsafe { pet_get_state(self.pet_handle, &mut state_json_ptr); }
        let state_json = if state_json_ptr.is_null() {
            String::new()
        } else {
            let s = unsafe { std::ffi::CStr::from_ptr(state_json_ptr).to_string_lossy().into_owned() };
            unsafe { pet_free_string(state_json_ptr); }
            s
        };
        
        // Pega animação atual
        let mut anim_ptr = ptr::null_mut();
        unsafe { pet_get_animation(self.pet_handle, &mut anim_ptr); }
        let animation = if anim_ptr.is_null() {
            "idle".to_string()
        } else {
            let a = unsafe { std::ffi::CStr::from_ptr(anim_ptr).to_string_lossy().into_owned() };
            unsafe { pet_free_string(anim_ptr); }
            a
        };
        
        // Pega fala se houver
        let mut speech_ptr = ptr::null_mut();
        if unsafe { pet_get_speech(self.pet_handle, &mut speech_ptr) } == 0 {
            if !speech_ptr.is_null() {
                let text = unsafe { std::ffi::CStr::from_ptr(speech_ptr).to_string_lossy().into_owned() };
                unsafe { pet_free_string(speech_ptr); }
                *self.speech_bubble.lock().unwrap() = Some(text);
                *self.speech_timer.lock().unwrap() = Instant::now();
            }
        }
        
        // Remove fala antiga (>5s)
        {
            let mut bubble = self.speech_bubble.lock().unwrap();
            let mut timer = self.speech_timer.lock().unwrap();
            if let Some(_) = bubble.as_ref() {
                if timer.elapsed() > Duration::from_secs(5) {
                    *bubble = None;
                }
            }
        }
        
        // Parse state JSON para posição do pet
        let (pet_x, pet_y) = if let Ok(state) = serde_json::from_str::<serde_json::Value>(&state_json) {
            // Posição baseada no cursor ou centro
            let cursor = self.cursor_pos.lock().unwrap();
            (cursor.0 - 50, cursor.1 - 50)
        } else {
            let (vx, vy, vw, vh) = self.virtual_screen;
            (vx + vw / 2 - 50, vy + vh / 2 - 50)
        };
        
        // Render
        self.draw_frame(pet_x, pet_y, &animation, &state_json);
    }
    
    fn draw_frame(&mut self, x: i32, y: i32, animation: &str, state_json: &str) {
        let (_, _, vw, vh) = self.virtual_screen;
        let mut pixmap = match Pixmap::new(vw as u32, vh as u32) {
            Some(p) => p,
            None => return,
        };
        
        // Desenha sprite do pet
        if let Some(sheet) = &self.sprite_sheet {
            self.draw_sprite(&mut pixmap, sheet, x, y, animation);
        } else {
            // Fallback: círculo simples
            self.draw_fallback_pet(&mut pixmap, x, y);
        }
        
        // Desenha bolha de fala
        if let Some(text) = self.speech_bubble.lock().unwrap().as_ref() {
            self.draw_speech_bubble(&mut pixmap, x + 32, y - 60, text);
        }
        
        // UpdateLayeredWindowIndirect com dirty rect
        self.present(&pixmap, x, y);
    }
    
    fn draw_sprite(&self, pixmap: &mut Pixmap, sheet: &Pixmap, x: i32, y: i32, animation: &str) {
        // Frame size 64x64, calcula frame baseado no timer
        let frame_w = 64;
        let frame_h = 64;
        let cols = sheet.width() / frame_w;
        
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
        
        let src_x = (frame_idx % cols as usize) * frame_w;
        let src_y = (frame_idx / cols as usize) * frame_h;
        
        // Desenha com tiny-skia
        let mut paint = Paint::default();
        paint.anti_alias = false;
        
        let src_rect = tiny_skia::Rect::from_xywh(
            src_x as f32, src_y as f32, 
            frame_w as f32, frame_h as f32
        ).unwrap();
        
        let dst_rect = tiny_skia::Rect::from_xywh(
            x as f32, y as f32,
            frame_w as f32 * 1.5, frame_h as f32 * 1.5 // scale 1.5
        ).unwrap();
        
        pixmap.draw_image(
            sheet.as_image(),
            src_rect,
            dst_rect,
            paint,
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
        // Background da bolha
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 255, 255, 240));
        paint.anti_alias = true;
        
        // Mede texto (aprox)
        let char_w = 7.0;
        let padding = 12.0;
        let lines: Vec<&str> = text.lines().collect();
        let max_w = lines.iter().map(|l| l.len() as f32 * char_w).fold(0.0, f32::max);
        let bubble_w = max_w + padding * 2.0;
        let bubble_h = lines.len() as f32 * 20.0 + padding * 2.0;
        
        let bx = x as f32 - bubble_w / 2.0;
        let by = y as f32 - bubble_h - 10.0;
        
        // Retângulo arredondado
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
        
        // Texto (tiny-skia não tem text rendering nativo, usa fallback simples)
        // TODO: usar fontdue ou similar para texto real
    }
    
    fn present(&self, pixmap: &Pixmap, pet_x: i32, pet_y: i32) {
        // Dirty rect: área ao redor do pet + bolha
        let dirty_w = 200;
        let dirty_h = 200;
        let dirty_x = pet_x - 50;
        let dirty_y = pet_y - 150;
        
        // Converte para BGRA para UpdateLayeredWindow
        let mut bgra = vec![0u8; pixmap.width() as usize * pixmap.height() as usize * 4];
        for (i, pixel) in pixmap.pixels().enumerate() {
            let rgba = pixel.to_rgba8();
            bgra[i * 4] = rgba[2];     // B
            bgra[i * 4 + 1] = rgba[1]; // G
            bgra[i * 4 + 2] = rgba[0]; // R
            bgra[i * 4 + 3] = rgba[3]; // A
        }
        
        // Cria DIB
        let mut bmi: BITMAPINFO = unsafe { mem::zeroed() };
        bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = pixmap.width() as i32;
        bmi.bmiHeader.biHeight = -(pixmap.height() as i32); // Top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        
        let hdc_screen = unsafe { GetDC(HWND(0)) };
        let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };
        let hbitmap = unsafe { CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, None, None, 0) };
        let old_bitmap = unsafe { SelectObject(hdc_mem, hbitmap) };
        
        // Copia pixels
        unsafe {
            let dst = SelectObject(hdc_mem, hbitmap);
            let src_ptr = bgra.as_ptr() as *const c_void;
            let dst_ptr = GetBitmapBits(hbitmap, (bgra.len()) as i32) as *mut c_void;
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, bgra.len());
        }
        
        // UpdateLayeredWindow
        let mut pt_src = POINT { x: 0, y: 0 };
        let mut pt_dst = POINT { x: self.virtual_screen.0, y: self.virtual_screen.1 };
        let mut size = SIZE { cx: pixmap.width() as i32, cy: pixmap.height() as i32 };
        let mut blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER.0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA.0,
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
                ULW_ALPHA.0,
            );
        }
        
        // Cleanup
        unsafe {
            SelectObject(hdc_mem, old_bitmap);
            DeleteObject(hbitmap);
            DeleteDC(hdc_mem);
            ReleaseDC(HWND(0), hdc_screen);
        }
    }

    fn run_loop(&mut self) {
        let mut msg = MSG::default();
        
        while self.running.load(Ordering::Relaxed) {
            // Processa mensagens Windows
            while unsafe { PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).into() } {
                if msg.message == WM_QUIT {
                    self.running.store(false, Ordering::Relaxed);
                    break;
                }
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            
            // Render frame
            self.render_frame();
            
            // Sleep curto
            thread::sleep(Duration::from_millis(16));
        }
    }
}

// Window procedure
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
                
                // Notifica pet sobre cursor (hover = carinho potencial)
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
                    unsafe { pet_interact(state.pet_handle, 1); } // Play
                }
            }
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if state_ptr != 0 {
                let state = unsafe { &mut *(state_ptr as *mut OverlayState) };
                if !state.pet_handle.is_null() {
                    unsafe { pet_interact(state.pet_handle, 0); } // Pet
                }
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            if wparam.0 == 1 { // Nosso hotkey de pânico
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
    // Inicializa tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    tracing::info!("Starting Pet Bolha overlay...");
    
    // Cria pet via FFI
    let name = CString::new("Bolha")?;
    let personality = CString::new("balanced")?;
    let config_json = CString::new(r#"{"speech_enabled":true,"neural_enabled":true}"#)?;
    
    let pet_handle = unsafe { 
        pet_create(name.as_ptr(), personality.as_ptr(), config_json.as_ptr())
    };
    
    if pet_handle.is_null() {
        anyhow::bail!("Failed to create pet");
    }
    
    // Carrega pesos neurais se existirem
    unsafe { pet_load(pet_handle); }
    
    // Setup overlay
    let mut overlay = OverlayState::new(pet_handle)?;
    OverlayState::register_class()?;
    overlay.create_window()?;
    
    // Carrega sprite sheet
    let assets_dir = std::env::current_exe()?.parent().unwrap().join("assets").join("sprites");
    let sprite_path = assets_dir.join("bolha.png");
    if sprite_path.exists() {
        overlay.load_sprite_sheet(&sprite_path)?;
    }
    
    // Mostra janela (invisível, click-through)
    unsafe { ShowWindow(overlay.hwnd, SW_SHOWNOACTIVATE); }
    
    // Loop principal
    overlay.run_loop();
    
    // Cleanup
    unsafe { 
        pet_save(pet_handle);
        pet_destroy(pet_handle);
        UnregisterHotKey(overlay.hwnd, 1);
    }
    
    tracing::info!("Pet Bolha overlay stopped");
    Ok(())
}