use std::cell::RefCell;
use std::fmt::Write;

pub const TAG_INT: u8 = 0;
pub const TAG_UINT: u8 = 1;
pub const TAG_FLOAT32: u8 = 2;
pub const TAG_FLOAT64: u8 = 3;
pub const TAG_BOOL: u8 = 4;
pub const TAG_CHAR: u8 = 5;
pub const TAG_RAW: u8 = 255;

thread_local! {
    static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
    static CAPTURE_OUTPUT: RefCell<bool> = const { RefCell::new(false) };
}

pub fn begin_capture() {
    CAPTURE_OUTPUT.with(|capture| *capture.borrow_mut() = true);
}

pub fn take_output() -> String {
    CAPTURE_OUTPUT.with(|capture| *capture.borrow_mut() = false);
    OUTPUT.with(|output| std::mem::take(&mut *output.borrow_mut()))
}

/// TODO: Replace this raw tagged-byte bridge with the proper Vinyl formatting runtime.
///
/// # Safety
/// `bytes` must point to at least `size` readable bytes, or be null only when `size` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vinyl_print_value(bytes: *const u8, size: usize, tag: u8, newline: u8) {
    let bytes = unsafe { std::slice::from_raw_parts(bytes, size) };
    let text = match tag {
        TAG_INT => format_signed(bytes),
        TAG_UINT => format_unsigned(bytes),
        TAG_FLOAT32 if bytes.len() >= 4 => {
            Some(f32::from_le_bytes(bytes[..4].try_into().unwrap()).to_string())
        }
        TAG_FLOAT64 if bytes.len() >= 8 => {
            Some(f64::from_le_bytes(bytes[..8].try_into().unwrap()).to_string())
        }
        TAG_BOOL => Some((bytes.first().copied().unwrap_or(0) != 0).to_string()),
        TAG_CHAR if bytes.len() >= 4 => {
            char::from_u32(u32::from_le_bytes(bytes[..4].try_into().unwrap()))
                .map(|value| value.to_string())
        }
        _ => Some(format_hex(bytes)),
    }
    .unwrap_or_else(|| format_hex(bytes));

    if CAPTURE_OUTPUT.with(|capture| *capture.borrow()) {
        OUTPUT.with(|output| {
            let mut output = output.borrow_mut();
            output.push_str(&text);
            if newline != 0 {
                output.push('\n');
            }
        });
    } else if newline != 0 {
        println!("{text}");
    } else {
        print!("{text}");
    }
}

fn format_signed(bytes: &[u8]) -> Option<String> {
    Some(match bytes.len() {
        1 => i8::from_le_bytes([bytes[0]]).to_string(),
        2 => i16::from_le_bytes(bytes.try_into().ok()?).to_string(),
        4 => i32::from_le_bytes(bytes.try_into().ok()?).to_string(),
        8 => i64::from_le_bytes(bytes.try_into().ok()?).to_string(),
        16 => i128::from_le_bytes(bytes.try_into().ok()?).to_string(),
        _ => return None,
    })
}

fn format_unsigned(bytes: &[u8]) -> Option<String> {
    Some(match bytes.len() {
        1 => u8::from_le_bytes([bytes[0]]).to_string(),
        2 => u16::from_le_bytes(bytes.try_into().ok()?).to_string(),
        4 => u32::from_le_bytes(bytes.try_into().ok()?).to_string(),
        8 => u64::from_le_bytes(bytes.try_into().ok()?).to_string(),
        16 => u128::from_le_bytes(bytes.try_into().ok()?).to_string(),
        _ => return None,
    })
}

fn format_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            text.push(' ');
        }
        write!(text, "{byte:02x}").unwrap();
    }
    text
}
