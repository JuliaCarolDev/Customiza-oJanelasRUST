use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Input::KeyboardAndMouse::*,
};
use std::io::{self, Write};

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
    if length == 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; (length + 1) as usize];
    unsafe {
        GetWindowTextW(hwnd, &mut buffer);
    }
    String::from_utf16_lossy(&buffer).trim_end_matches('\0').to_string()
}

fn get_window_class(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    unsafe {
        let len = GetClassNameW(hwnd, &mut buffer);
        if len > 0 {
            return String::from_utf16_lossy(&buffer[..len as usize]);
        }
    }
    String::new()
}

fn get_window_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut rect);
    }
    rect
}

fn list_windows() -> Vec<WindowInfo> {
    let mut list = HwndList::new(1024);
    let list_ptr = &mut list as *mut HwndList;
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), LPARAM(list_ptr as isize));
    }
    list.hwnds
        .into_iter()
        .map(|hwnd| {
            WindowInfo {
                hwnd,
                title: get_window_title(hwnd),
                class: get_window_class(hwnd),
                rect: get_window_rect(hwnd),
            }
        })
        .collect()
}

fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOP, x, y, width, height, SWP_SHOWWINDOW);
    }
}

fn minimize_window(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
}

fn maximize_window(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_MAXIMIZE);
    }
}

fn restore_window(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    }
}

fn close_window(hwnd: HWND) {
    unsafe {
        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

fn bring_to_front(hwnd: HWND) {
    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
}

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
    println!("\n🔊 [ATALHO] Todas as janelas minimizadas!");
}

// ─── Thread de escuta de atalhos globais ───
fn start_hotkey_listener() {
    std::thread::spawn(|| {
        unsafe {
            // Registra: Win + T (Tile)
            let _ = RegisterHotKey(HWND::default(), HOTKEY_TILE, MOD_WIN, VK_T.0 as u32);
            // Registra: Win + C (Cascade)
            let _ = RegisterHotKey(HWND::default(), HOTKEY_CASCADE, MOD_WIN, VK_J.0 as u32);
            // Registra: Win + M (Minimize All)
            let _ = RegisterHotKey(HWND::default(), HOTKEY_MINIMIZE_ALL, MOD_WIN, VK_M.0 as u32);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
                if msg.message == WM_HOTKEY {
                    let hotkey_id = msg.wParam.0 as i32;
                    match hotkey_id {
                        HOTKEY_TILE => {
                            println!("\n⚡ [ATALHO] Organizando janelas em Grade (Tile)...");
                            let windows = list_windows();
                            tile_windows(&windows, 2, 2);
                        }
                        HOTKEY_CASCADE => {
                            println!("\n⚡ [ATALHO] Organizando janelas em Cascata...");
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

            // Limpeza ao fechar a thread
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_TILE);
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_CASCADE);
            let _ = UnregisterHotKey(HWND::default(), HOTKEY_MINIMIZE_ALL);
        }
    });
}

fn print_menu() {
    println!("\n╔══════════════════════════════════════════╗");
    println!("║  🪟  WINDOW MANAGER - RUST (Com Atalhos) ║");
    println!("══════════════════════════════════════════╣");
    println!("║  1. Listar janelas                       ║");
    println!("║  2. Mover/Redimensionar janela           ║");
    println!("║  3. Minimizar janela                     ║");
    println!("║  4. Maximizar janela                     ║");
    println!("║  5. Restaurar janela                     ║");
    println!("║  6. Fechar janela                        ║");
    println!("║  7. Trazer para frente                   ║");
    println!("║  8. Tile (organizar em grade)            ║");
    println!("║  9. Cascade (organizar em cascata)       ║");
    println!("║  0. Sair                                 ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  ⌨️  Win+T: Tile | Win+J: Cascade        ║");
    println!("║  ⌨️  Win+M: Minimizar Tudo               ║");
    println!("╚══════════════════════════════════════════╝");
    print!("Escolha uma opção: ");
    io::stdout().flush().unwrap();
}

fn main() {
    println!("🚀 Window Manager iniciado!\n");

    // Inicia o listener de atalhos em segundo plano
    start_hotkey_listener();

    loop {
        print_menu();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).ok();
        let choice = choice.trim();

        match choice {
            "1" => {
                let windows = list_windows();
                println!("\n📋 Janelas encontradas ({}):\n", windows.len());
                for (i, win) in windows.iter().enumerate() {
                    println!(
                        "  [{}] \"{}\" (classe: {}) - Pos: ({}, {}), Tamanho: {}x{}",
                        i,
                        win.title,
                        win.class,
                        win.rect.left,
                        win.rect.top,
                        win.rect.right - win.rect.left,
                        win.rect.bottom - win.rect.top,
                    );
                }
                println!();
            }
            "2" => {
                let windows = list_windows();
                print!("Índice da janela: ");
                io::stdout().flush().unwrap();
                let mut idx = String::new();
                std::io::stdin().read_line(&mut idx).ok();
                let idx: usize = idx.trim().parse().unwrap_or(0);

                if let Some(win) = windows.get(idx) {
                    print!("X: ");
                    io::stdout().flush().unwrap();
                    let mut x = String::new();
                    std::io::stdin().read_line(&mut x).ok();
                    let x: i32 = x.trim().parse().unwrap_or(0);

                    print!("Y: ");
                    io::stdout().flush().unwrap();
                    let mut y = String::new();
                    std::io::stdin().read_line(&mut y).ok();
                    let y: i32 = y.trim().parse().unwrap_or(0);

                    print!("Largura: ");
                    io::stdout().flush().unwrap();
                    let mut w = String::new();
                    std::io::stdin().read_line(&mut w).ok();
                    let w: i32 = w.trim().parse().unwrap_or(800);

                    print!("Altura: ");
                    io::stdout().flush().unwrap();
                    let mut h = String::new();
                    std::io::stdin().read_line(&mut h).ok();
                    let h: i32 = h.trim().parse().unwrap_or(600);

                    move_window(win.hwnd, x, y, w, h);
                    println!("✅ Janela movida!");
                } else {
                    println!("❌ Índice inválido!");
                }
            }
            "3" => {
                let windows = list_windows();
                print!("Índice da janela: ");
                io::stdout().flush().unwrap();
                let mut idx = String::new();
                std::io::stdin().read_line(&mut idx).ok();
                let idx: usize = idx.trim().parse().unwrap_or(0);
                if let Some(win) = windows.get(idx) {
                    minimize_window(win.hwnd);
                    println!("✅ Janela minimizada!");
                } else {
                    println!("❌ Índice inválido!");
                }
            }
            "4" => {
                let windows = list_windows();
                print!("Índice da janela: ");
                io::stdout().flush().unwrap();
                let mut idx = String::new();
                std::io::stdin().read_line(&mut idx).ok();
                let idx: usize = idx.trim().parse().unwrap_or(0);
                if let Some(win) = windows.get(idx) {
                    maximize_window(win.hwnd);
                    println!("✅ Janela maximizada!");
                } else {
                    println!("❌ Índice inválido!");
                }
            }
            "5" => {
                let windows = list_windows();
                print!("Índice da janela: ");
                io::stdout().flush().unwrap();
                let mut idx = String::new();
                std::io::stdin().read_line(&mut idx).ok();
                let idx: usize = idx.trim().parse().unwrap_or(0);
                if let Some(win) = windows.get(idx) {
                    restore_window(win.hwnd);
                    println!("✅ Janela restaurada!");
                } else {
                    println!("❌ Índice inválido!");
                }
            }
            "6" => {
                let windows = list_windows();
                print!("Índice da janela: ");
                io::stdout().flush().unwrap();
                let mut idx = String::new();
                std::io::stdin().read_line(&mut idx).ok();
                let idx: usize = idx.trim().parse().unwrap_or(0);
                if let Some(win) = windows.get(idx) {
                    close_window(win.hwnd);
                    println!("✅ Janela fechada!");
                } else {
                    println!(" Índice inválido!");
                }
            }
            "7" => {
                let windows = list_windows();
                print!("Índice da janela: ");
                io::stdout().flush().unwrap();
                let mut idx = String::new();
                std::io::stdin().read_line(&mut idx).ok();
                let idx: usize = idx.trim().parse().unwrap_or(0);
                if let Some(win) = windows.get(idx) {
                    bring_to_front(win.hwnd);
                    println!("✅ Janela trazida para frente!");
                } else {
                    println!("❌ Índice inválido!");
                }
            }
            "8" => {
                let windows = list_windows();
                tile_windows(&windows, 2, 2);
                println!("✅ Janelas organizadas em tile!");
            }
            "9" => {
                let windows = list_windows();
                cascade_windows(&windows);
                println!("✅ Janelas organizadas em cascata!");
            }
            "0" => {
                println!(" Saindo e limpando atalhos...");
                break;
            }
            _ => {
                println!("❌ Opção inválida!");
            }
        }
    }
}