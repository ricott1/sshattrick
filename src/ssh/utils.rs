use crate::types::TerminalEvent;
use anyhow::anyhow;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

pub const CMD_RESIZE: u8 = 0x04;
const SGR_MOUSE_PREFIX: &[u8] = b"\x1b[<";

fn convert_data_to_key_event(data: &[u8]) -> Option<crossterm::event::KeyEvent> {
    let key = match data {
        b"\x1b\x5b\x41" => crossterm::event::KeyCode::Up,
        b"\x1b\x5b\x42" => crossterm::event::KeyCode::Down,
        b"\x1b\x5b\x43" => crossterm::event::KeyCode::Right,
        b"\x1b\x5b\x44" => crossterm::event::KeyCode::Left,
        b"\x03" | b"\x1b" => crossterm::event::KeyCode::Esc,
        b"\x0d" => crossterm::event::KeyCode::Enter,
        b"\x7f" => crossterm::event::KeyCode::Backspace,
        b"\x1b[3~" => crossterm::event::KeyCode::Delete,
        b"\x09" => crossterm::event::KeyCode::Tab,
        x if x.len() == 1 => crossterm::event::KeyCode::Char(data[0] as char),
        _ => return None,
    };
    Some(crossterm::event::KeyEvent::new(
        key,
        crossterm::event::KeyModifiers::empty(),
    ))
}

fn decode_sgr_mouse_input(ansi_code: &[u8]) -> anyhow::Result<(u8, u16, u16)> {
    let ansi_str = std::str::from_utf8(ansi_code).map_err(|_| anyhow!("Invalid UTF-8 sequence"))?;

    if !ansi_str.as_bytes().starts_with(SGR_MOUSE_PREFIX) {
        return Err(anyhow!("Invalid SGR ANSI mouse code"));
    }

    let cb_mod = if ansi_str.ends_with('M') {
        0
    } else if ansi_str.ends_with('m') {
        3
    } else {
        return Err(anyhow!("Invalid SGR ANSI mouse code"));
    };

    let code_body = &ansi_str[3..ansi_str.len() - 1];
    let components: Vec<&str> = code_body.split(';').collect();
    if components.len() != 3 {
        return Err(anyhow!("Invalid SGR ANSI mouse code format"));
    }

    let cb = cb_mod
        + components[0]
            .parse::<u8>()
            .map_err(|_| anyhow!("Failed to parse Cb"))?;
    let cx = components[1]
        .parse::<u16>()
        .map_err(|_| anyhow!("Failed to parse Cx"))?
        - 1;
    let cy = components[2]
        .parse::<u16>()
        .map_err(|_| anyhow!("Failed to parse Cy"))?
        - 1;

    Ok((cb, cx, cy))
}

fn convert_data_to_mouse_event(data: &[u8]) -> Option<crossterm::event::MouseEvent> {
    let (cb, column, row) = decode_sgr_mouse_input(data).ok()?;
    let kind = match cb {
        0 => crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        1 => crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Middle),
        2 => crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right),
        3 => crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
        32 => crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
        33 => crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Middle),
        34 => crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Right),
        35 => crossterm::event::MouseEventKind::Moved,
        64 => crossterm::event::MouseEventKind::ScrollUp,
        65 => crossterm::event::MouseEventKind::ScrollDown,
        _ => return None,
    };

    Some(crossterm::event::MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

pub fn convert_data_to_terminal_event(data: &[u8]) -> Option<TerminalEvent> {
    if let Some(&[cols, rows]) = data.strip_prefix(&[CMD_RESIZE]) {
        return Some(TerminalEvent::Resize(cols as u16, rows as u16));
    }

    if data.starts_with(SGR_MOUSE_PREFIX) {
        return convert_data_to_mouse_event(data).map(TerminalEvent::Mouse);
    }

    let key = convert_data_to_key_event(data)?;
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.code == KeyCode::Esc {
        return Some(TerminalEvent::Quit);
    }
    Some(TerminalEvent::Key(key))
}
