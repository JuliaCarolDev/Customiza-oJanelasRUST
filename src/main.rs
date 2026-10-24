use eframe::egui;
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Input::KeyboardAndMouse::*,
};

// ─── IDs únicos para nossos atalhos (i32!) ───
const HOTKEY_TILE: i32 = 1;
const HOTKEY_CASCADE: i32 = 2;
const HOTKEY_MINIMIZE_ALL: i32 = 3;

#[derive(Debug, Clone)]
struct WindowInfo {
    hwnd: HWND,
    title: String,
    class: String,
    rect: RECT,
}

struct HwndList {
    hwnds: Vec<HWND>,
    capacity: usize,
}

impl HwndList {
    fn new(capacity: usize) -> Self {
        Self {
            hwnds: Vec::with_capacity(capacity),
            capacity,
        }
    }
    fn push(&mut self, hwnd: HWND) {
        if self.hwnds.len() < self.capacity {
            self.hwnds.push(hwnd);
        }
    }
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list: &mut HwndList = &mut *(lparam.0 as *mut HwndList);
    if IsWindowVisible(hwnd).as_bool() {
        let length = GetWindowTextLengthW(hwnd);
        if length > 0 {
            list.push(hwnd);
        }
    }
    BOOL(1)
}

fn get_window_title(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length == 0 { return String::new(); }
    let mut buffer = vec![0u16; (length + 1) as usize];
    unsafe { GetWindowTextW(hwnd, &mut buffer); }
    String::from_utf16_lossy(&buffer).trim_end_matches('\0').to_string()
}

fn get_window_class(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    unsafe {
        let len = GetClassNameW(hwnd, &mut buffer);
        if len > 0 { return String::from_utf16_lossy(&buffer[..len as usize]); }
    }
    String::new()
}

fn get_window_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    unsafe { let _ = GetWindowRect(hwnd, &mut rect); }
    rect
}

fn list_windows() -> Vec<WindowInfo> {
    let mut list = HwndList::new(1024);
    let list_ptr = &mut list as *mut HwndList;
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(list_ptr as isize));
    }
    list.hwnds.into_iter().map(|hwnd| {
        WindowInfo {
            hwnd,
            title: get_window_title(hwnd),
            class: get_window_class(hwnd),
            rect: get_window_rect(hwnd),
        }
    }).collect()
}

// ─── Ações de Janela ───
fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    unsafe { let _ = SetWindowPos(hwnd, HWND_TOP, x, y, width, height, SWP_SHOWWINDOW); }
}
fn minimize_window(hwnd: HWND) { unsafe { let _ = ShowWindow(hwnd, SW_MINIMIZE); } }
fn maximize_window(hwnd: HWND) { unsafe { let _ = ShowWindow(hwnd, SW_MAXIMIZE); } }
fn restore_window(hwnd: HWND) { unsafe { let _ = ShowWindow(hwnd, SW_RESTORE); } }
fn close_window(hwnd: HWND) { unsafe { let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)); } }
fn bring_to_front(hwnd: HWND) { unsafe { let _ = SetForegroundWindow(hwnd); } }

fn tile_windows(windows: &[WindowInfo], cols: i32, rows: i32) {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let tile_width = screen_width / cols;
    let tile_height = screen_height / rows;

    for (i, win) in windows.iter().enumerate() {
        let col = (i as i32) % cols;
        let row = (i as i32) / cols;
        move_window(win.hwnd, col * tile_width, row * tile_height, tile_width, tile_height);
    }
}

fn cascade_windows(windows: &[WindowInfo]) {
    let offset = 40i32;
    let width = 800i32;
    let height = 600i32;
    for (i, win) in windows.iter().enumerate() {
        move_window(win.hwnd, (i as i32) * offset, (i as i32) * offset, width, height);
    }
}

fn minimize_all_windows() {
    let windows = list_windows();
    for win in windows.iter() {
        if win.class != "Shell_TrayWnd" && win.class != "Progman" {
            minimize_window(win.hwnd);
        }
    }
}

// ─── Thread de escuta de atalhos globais ───
fn start_hotkey_listener() {
    std::thread::spawn(|| {
        unsafe {
            let _ = RegisterHotKey(HWND::default(), HOTKEY_TILE, MOD_WIN, VK_T.0 as u32);
            let _ = RegisterHotKey(HWND::default(), HOTKEY_CASCADE, MOD_WIN, VK_J.0 as u32);
            let _ = RegisterHotKey(HWND::default(), HOTKEY_MINIMIZE_ALL, MOD_WIN, VK_M.0 as u32);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
                if msg.message == WM_HOTKEY {
                    let hotkey_id = msg.wParam.0 as i32;
                    match hotkey_id {
                        HOTKEY_TILE => {
                            let windows = list_windows();
                            tile_windows(&windows, 2, 2);
                        }
                        HOTKEY_CASCADE => {
                            let windows = list_windows();
                            cascade_windows(&windows);
                        }
                        HOTKEY_MINIMIZE_ALL => {
                            minimize_all_windows();
                        }
                        _ => {}
                    }
                }
            }
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_TILE);
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_CASCADE);
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_MINIMIZE_ALL);
        }
    });
}

// ─── Aplicação GUI com eframe/egui ───
struct WindowManagerApp {
    windows: Vec<WindowInfo>,
}

impl WindowManagerApp {
    fn new() -> Self {
        let mut app = Self { windows: Vec::new() };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.windows = list_windows();
    }
}

impl eframe::App for WindowManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut needs_refresh = false; // 🔑 Flag para controlar atualização

        // 1. Barra Superior
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🪟 Window Manager");
                ui.add_space(20.0);
                if ui.button("🔄 Atualizar Lista").clicked() {
                    needs_refresh = true;
                }
                ui.label(" | ");
                ui.label("Atalhos: Win+T (Tile) | Win+J (Cascade) | Win+M (Minimizar Tudo)");
            });
        });

        // 2. Painel Central (Lista de Janelas)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Total de janelas encontradas: {}", self.windows.len()));
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, win) in self.windows.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:3}", i));
                        ui.label(format!("\"{}\"", win.title));
                        ui.label(egui::RichText::new(format!("(classe: {})", win.class)).weak());
                        
                        // Botões de ação alinhados à direita
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("🎯 Foco").clicked() { 
                                bring_to_front(win.hwnd); 
                                needs_refresh = true; //  Marca para atualizar depois
                            }
                            if ui.small_button("⬆ Max").clicked() { 
                                maximize_window(win.hwnd); 
                                needs_refresh = true;
                            }
                            if ui.small_button("⬇ Min").clicked() { 
                                minimize_window(win.hwnd); 
                                needs_refresh = true;
                            }
                            if ui.small_button("❌ Fechar").clicked() { 
                                close_window(win.hwnd); 
                                needs_refresh = true;
                            }
                        });
                    });
                    ui.separator();
                }
            });
        });

        // 3. Barra Inferior (Ações Globais)
        egui::TopBottomPanel::bottom("actions").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                if ui.button("📐 Tile (2x2)").clicked() { 
                    tile_windows(&self.windows, 2, 2); 
                    needs_refresh = true;
                }
                if ui.button("🌊 Cascade").clicked() { 
                    cascade_windows(&self.windows); 
                    needs_refresh = true;
                }
                if ui.button("💤 Minimizar Tudo").clicked() { 
                    minimize_all_windows(); 
                    needs_refresh = true;
                }
            });
        });

        // 🔑 Atualiza a lista SE necessário (depois que todos os borrows terminaram)
        if needs_refresh {
            self.refresh();
        }
    }
}

// ─── Ponto de Entrada ───
fn main() -> Result<(), eframe::Error> {
    println!("🚀 Iniciando Window Manager em segundo plano...");
    
    // Inicia os atalhos globais em uma thread separada
    start_hotkey_listener();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title("🪟 Window Manager - Rust"),
        ..Default::default()
    };

    eframe::run_native(
        "Window Manager",
        options,
        Box::new(|_cc| Ok(Box::new(WindowManagerApp::new()))),
    )
}