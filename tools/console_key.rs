use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct KeyEventRecord {
    key_down: i32,
    repeat_count: u16,
    virtual_key_code: u16,
    virtual_scan_code: u16,
    unicode_char: u16,
    control_key_state: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputRecord {
    event_type: u16,
    padding: u16,
    key_event: KeyEventRecord,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn FreeConsole() -> i32;
    fn AttachConsole(process_id: u32) -> i32;
    fn GetLastError() -> u32;
    fn GetStdHandle(std_handle: u32) -> *mut c_void;
    fn WriteConsoleInputW(
        input: *mut c_void,
        records: *const InputRecord,
        length: u32,
        written: *mut u32,
    ) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn MapVirtualKeyW(code: u32, map_type: u32) -> u32;
}

fn main() {
    let pid = std::env::args()
        .nth(1)
        .expect("usage: console_key <pid> <left|right|up|down>")
        .parse::<u32>()
        .expect("pid must be an integer");
    let key_name = std::env::args()
        .nth(2)
        .expect("usage: console_key <pid> <left|right|up|down|tab|f6|plus|minus>");
    let (virtual_key, unicode_char, control_key_state) = match key_name.as_str() {
        "left" => (0x25, 0, 0x0100),
        "up" => (0x26, 0, 0x0100),
        "right" => (0x27, 0, 0x0100),
        "down" => (0x28, 0, 0x0100),
        "tab" => (0x09, 9, 0),
        "f6" => (0x75, 0, 0),
        "plus" => (0x6b, '+' as u16, 0),
        "minus" => (0x6d, '-' as u16, 0),
        _ => panic!("unsupported key: {key_name}"),
    };

    // SAFETY: The records have the documented Win32 INPUT_RECORD layout and
    // are written only to the console input buffer owned by the supplied PID.
    unsafe {
        FreeConsole();
        if AttachConsole(pid) == 0 {
            panic!("AttachConsole failed with Windows error {}", GetLastError());
        }
        let input = GetStdHandle((-10i32) as u32);
        let scan_code = MapVirtualKeyW(virtual_key, 0) as u16;
        let records = [
            InputRecord {
                event_type: 1,
                padding: 0,
                key_event: KeyEventRecord {
                    key_down: 1,
                    repeat_count: 1,
                    virtual_key_code: virtual_key as u16,
                    virtual_scan_code: scan_code,
                    unicode_char,
                    control_key_state,
                },
            },
            InputRecord {
                event_type: 1,
                padding: 0,
                key_event: KeyEventRecord {
                    key_down: 0,
                    repeat_count: 1,
                    virtual_key_code: virtual_key as u16,
                    virtual_scan_code: scan_code,
                    unicode_char,
                    control_key_state,
                },
            },
        ];
        let mut written = 0u32;
        if WriteConsoleInputW(input, records.as_ptr(), records.len() as u32, &mut written) == 0 {
            panic!(
                "WriteConsoleInputW failed with Windows error {}",
                GetLastError()
            );
        }
        assert_eq!(written, records.len() as u32, "not all key records were written");
    }
}
