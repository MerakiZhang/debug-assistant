pub fn text_input_char(text: &mut String, cursor: &mut usize, c: char) {
    text.insert(*cursor, c);
    *cursor += c.len_utf8();
}

pub fn text_backspace(text: &mut String, cursor: &mut usize) {
    if *cursor > 0 {
        let pos = text[..*cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        text.remove(pos);
        *cursor = pos;
    }
}

pub fn text_cursor_left(text: &str, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    *cursor = text[..*cursor]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0);
}

pub fn text_cursor_right(text: &str, cursor: &mut usize) {
    if *cursor < text.len() {
        let c = text[*cursor..].chars().next().unwrap();
        *cursor += c.len_utf8();
    }
}

pub fn text_cursor_home(cursor: &mut usize) {
    *cursor = 0;
}

pub fn text_cursor_end(text: &str, cursor: &mut usize) {
    *cursor = text.len();
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- text_input_char ---

    #[test]
    fn input_char_appends_at_end() {
        let mut s = String::from("ab");
        let mut cur = 2;
        text_input_char(&mut s, &mut cur, 'c');
        assert_eq!(s, "abc");
        assert_eq!(cur, 3);
    }

    #[test]
    fn input_char_inserts_at_start() {
        let mut s = String::from("bc");
        let mut cur = 0;
        text_input_char(&mut s, &mut cur, 'a');
        assert_eq!(s, "abc");
        assert_eq!(cur, 1);
    }

    #[test]
    fn input_char_inserts_in_middle() {
        let mut s = String::from("ac");
        let mut cur = 1;
        text_input_char(&mut s, &mut cur, 'b');
        assert_eq!(s, "abc");
        assert_eq!(cur, 2);
    }

    #[test]
    fn input_char_multibyte() {
        let mut s = String::new();
        let mut cur = 0;
        text_input_char(&mut s, &mut cur, '中');
        assert_eq!(s, "中");
        assert_eq!(cur, '中'.len_utf8());
    }

    // --- text_backspace ---

    #[test]
    fn backspace_removes_last_char() {
        let mut s = String::from("abc");
        let mut cur = 3;
        text_backspace(&mut s, &mut cur);
        assert_eq!(s, "ab");
        assert_eq!(cur, 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut s = String::from("abc");
        let mut cur = 0;
        text_backspace(&mut s, &mut cur);
        assert_eq!(s, "abc");
        assert_eq!(cur, 0);
    }

    #[test]
    fn backspace_removes_multibyte_char() {
        let mut s = String::from("a中");
        let mut cur = s.len();
        text_backspace(&mut s, &mut cur);
        assert_eq!(s, "a");
        assert_eq!(cur, 1);
    }

    // --- text_cursor_left ---

    #[test]
    fn cursor_left_moves_back_one_char() {
        let s = "abc";
        let mut cur = 3;
        text_cursor_left(s, &mut cur);
        assert_eq!(cur, 2);
    }

    #[test]
    fn cursor_left_at_start_is_noop() {
        let s = "abc";
        let mut cur = 0;
        text_cursor_left(s, &mut cur);
        assert_eq!(cur, 0);
    }

    #[test]
    fn cursor_left_over_multibyte_char() {
        let s = "a中b";
        let mut cur = s.len(); // after 'b'
        text_cursor_left(s, &mut cur);
        assert_eq!(cur, 1 + '中'.len_utf8()); // before 'b'
    }

    // --- text_cursor_right ---

    #[test]
    fn cursor_right_moves_forward_one_char() {
        let s = "abc";
        let mut cur = 0;
        text_cursor_right(s, &mut cur);
        assert_eq!(cur, 1);
    }

    #[test]
    fn cursor_right_at_end_is_noop() {
        let s = "abc";
        let mut cur = 3;
        text_cursor_right(s, &mut cur);
        assert_eq!(cur, 3);
    }

    #[test]
    fn cursor_right_over_multibyte_char() {
        let s = "中b";
        let mut cur = 0;
        text_cursor_right(s, &mut cur);
        assert_eq!(cur, '中'.len_utf8());
    }

    // --- text_cursor_home ---

    #[test]
    fn cursor_home_moves_to_zero() {
        let mut cur = 5;
        text_cursor_home(&mut cur);
        assert_eq!(cur, 0);
    }

    #[test]
    fn cursor_home_already_at_zero() {
        let mut cur = 0;
        text_cursor_home(&mut cur);
        assert_eq!(cur, 0);
    }

    // --- text_cursor_end ---

    #[test]
    fn cursor_end_moves_to_end() {
        let s = "hello";
        let mut cur = 0;
        text_cursor_end(s, &mut cur);
        assert_eq!(cur, 5);
    }

    #[test]
    fn cursor_end_already_at_end() {
        let s = "hello";
        let mut cur = 5;
        text_cursor_end(s, &mut cur);
        assert_eq!(cur, 5);
    }
}
