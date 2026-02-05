use opal_abi::msg::event::KeyCode;

/// Given a keycode returns all the possible characters that can be typed with that key:
///
/// returns Some((normal char, capslock char, shift char)) if keycode can be mapped to a character.
pub const fn keycode_to_char(code: KeyCode) -> Option<(char, char, char)> {
    match code {
        // letters
        KeyCode::KeyA => Some(('a', 'A', 'A')),
        KeyCode::KeyB => Some(('b', 'B', 'B')),
        KeyCode::KeyC => Some(('c', 'C', 'C')),
        KeyCode::KeyD => Some(('d', 'D', 'D')),
        KeyCode::KeyE => Some(('e', 'E', 'E')),
        KeyCode::KeyF => Some(('f', 'F', 'F')),
        KeyCode::KeyG => Some(('g', 'G', 'G')),
        KeyCode::KeyH => Some(('h', 'H', 'H')),
        KeyCode::KeyI => Some(('i', 'I', 'I')),
        KeyCode::KeyJ => Some(('j', 'J', 'J')),
        KeyCode::KeyK => Some(('k', 'K', 'K')),
        KeyCode::KeyL => Some(('l', 'L', 'L')),
        KeyCode::KeyM => Some(('m', 'M', 'M')),
        KeyCode::KeyN => Some(('n', 'N', 'N')),
        KeyCode::KeyO => Some(('o', 'O', 'O')),
        KeyCode::KeyP => Some(('p', 'P', 'P')),
        KeyCode::KeyQ => Some(('q', 'Q', 'Q')),
        KeyCode::KeyR => Some(('r', 'R', 'R')),
        KeyCode::KeyS => Some(('s', 'S', 'S')),
        KeyCode::KeyT => Some(('t', 'T', 'T')),
        KeyCode::KeyU => Some(('u', 'U', 'U')),
        KeyCode::KeyV => Some(('v', 'V', 'V')),
        KeyCode::KeyW => Some(('w', 'W', 'W')),
        KeyCode::KeyX => Some(('x', 'X', 'X')),
        KeyCode::KeyY => Some(('y', 'Y', 'Y')),
        KeyCode::KeyZ => Some(('z', 'Z', 'Z')),

        // digits
        KeyCode::Key0 => Some(('0', ')', '0')),
        KeyCode::Key1 => Some(('1', '!', '1')),
        KeyCode::Key2 => Some(('2', '@', '2')),
        KeyCode::Key3 => Some(('3', '#', '3')),
        KeyCode::Key4 => Some(('4', '$', '4')),
        KeyCode::Key5 => Some(('5', '%', '5')),
        KeyCode::Key6 => Some(('6', '^', '6')),
        KeyCode::Key7 => Some(('7', '&', '7')),
        KeyCode::Key8 => Some(('8', '*', '8')),
        KeyCode::Key9 => Some(('9', '(', '9')),

        KeyCode::Space => Some((' ', ' ', ' ')),
        KeyCode::Comma => Some((',', '<', ',')),
        KeyCode::Dot => Some(('.', '<', '.')),
        KeyCode::Slash => Some(('/', '?', '/')),
        KeyCode::Semicolon => Some((';', ':', ';')),
        KeyCode::BackQuote => Some(('`', '~', '`')),
        KeyCode::LeftBrace => Some(('[', '{', '[')),
        KeyCode::RightBrace => Some((']', '}', ']')),
        KeyCode::BackSlash => Some(('\\', '|', '\\')),
        KeyCode::Minus => Some(('-', '_', '-')),
        KeyCode::Equals => Some(('=', '+', '=')),
        // FIXME: it isn't a double quote is it?
        KeyCode::DoubleQuote => Some(('\'', '"', '\'')),

        _ => None,
    }
}
