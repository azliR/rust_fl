use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use fl_rust::devices::selector::is_enter_key;

#[test]
fn test_is_enter_key_standard_enter() {
    let key = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&key));
}

#[test]
fn test_is_enter_key_newline_characters() {
    let newline = KeyEvent {
        code: KeyCode::Char('\n'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&newline));

    let carriage_return = KeyEvent {
        code: KeyCode::Char('\r'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&carriage_return));
}

#[test]
fn test_is_enter_key_control_j_and_m() {
    let ctrl_j = KeyEvent {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&ctrl_j));

    let ctrl_upper_j = KeyEvent {
        code: KeyCode::Char('J'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&ctrl_upper_j));

    let ctrl_m = KeyEvent {
        code: KeyCode::Char('m'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&ctrl_m));

    let ctrl_upper_m = KeyEvent {
        code: KeyCode::Char('M'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(is_enter_key(&ctrl_upper_m));
}

#[test]
fn test_is_enter_key_non_enter_keys() {
    let plain_j = KeyEvent {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(!is_enter_key(&plain_j));

    let plain_m = KeyEvent {
        code: KeyCode::Char('m'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(!is_enter_key(&plain_m));

    let digit_1 = KeyEvent {
        code: KeyCode::Char('1'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(!is_enter_key(&digit_1));

    let quit_q = KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(!is_enter_key(&quit_q));

    let ctrl_c = KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };
    assert!(!is_enter_key(&ctrl_c));
}
