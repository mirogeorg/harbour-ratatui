use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Coord {
    x: i16,
    y: i16,
}

#[repr(C)]
struct SmallRect {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
}

#[repr(C)]
struct ConsoleScreenBufferInfo {
    size: Coord,
    cursor_position: Coord,
    attributes: u16,
    window: SmallRect,
    maximum_window_size: Coord,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn FreeConsole() -> i32;
    fn AttachConsole(process_id: u32) -> i32;
    fn GetLastError() -> u32;
    fn GetStdHandle(std_handle: u32) -> *mut c_void;
    fn GetConsoleScreenBufferInfo(
        output: *mut c_void,
        info: *mut ConsoleScreenBufferInfo,
    ) -> i32;
    fn ReadConsoleOutputCharacterW(
        output: *mut c_void,
        characters: *mut u16,
        length: u32,
        position: Coord,
        characters_read: *mut u32,
    ) -> i32;
}

fn main() {
    let pid = std::env::args()
        .nth(1)
        .expect("usage: console_probe <pid>")
        .parse::<u32>()
        .expect("pid must be an integer");
    let report_path = std::env::args()
        .nth(2)
        .expect("usage: console_probe <pid> <report-path>");

    // SAFETY: These Win32 calls operate only on the console associated with
    // the supplied process. All output buffers below have their stated size.
    let screen = unsafe {
        FreeConsole();
        if AttachConsole(pid) == 0 {
            panic!("AttachConsole failed with Windows error {}", GetLastError());
        }
        let output = GetStdHandle((-11i32) as u32);
        let mut info = std::mem::zeroed::<ConsoleScreenBufferInfo>();
        if GetConsoleScreenBufferInfo(output, &mut info) == 0 {
            panic!("GetConsoleScreenBufferInfo failed");
        }

        let width = i32::from(info.window.right - info.window.left + 1) as usize;
        let mut screen = String::new();
        for y in info.window.top..=info.window.bottom {
            let mut row = vec![0u16; width];
            let mut read = 0u32;
            if ReadConsoleOutputCharacterW(
                output,
                row.as_mut_ptr(),
                width as u32,
                Coord {
                    x: info.window.left,
                    y,
                },
                &mut read,
            ) == 0
            {
                panic!("ReadConsoleOutputCharacterW failed");
            }
            screen.push_str(&String::from_utf16_lossy(&row[..read as usize]));
            screen.push('\n');
        }
        screen
    };

    let signatures = [
        "[0m", "[38;", "[48;", "[?25", "[2J", "[H", "Γö", "Γû", "╨", "πü",
    ];
    let found: Vec<&str> = signatures
        .iter()
        .copied()
        .filter(|signature| screen.contains(signature))
        .collect();

    let report = format!(
        "signatures={}\n{}",
        if found.is_empty() {
            "none".to_owned()
        } else {
            found.join(",")
        },
        screen
    );
    std::fs::write(report_path, report).expect("could not write console probe report");

    if !found.is_empty() {
        std::process::exit(2);
    }
}
