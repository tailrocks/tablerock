pub fn assert_fullscreen_lifecycle(output: &[u8]) {
    let sequences = [
        b"\x1b[?1049h".as_slice(),
        b"\x1b[?1000h".as_slice(),
        b"\x1b[?1002h".as_slice(),
        b"\x1b[?1003h".as_slice(),
        b"\x1b[?1015h".as_slice(),
        b"\x1b[?1006h".as_slice(),
        b"\x1b[?2004h".as_slice(),
        b"\x1b[?7l".as_slice(),
        b"\x1b[?25l".as_slice(),
        b"\x1b[?25h".as_slice(),
        b"\x1b[?7h".as_slice(),
        b"\x1b[?2004l".as_slice(),
        b"\x1b[?1006l".as_slice(),
        b"\x1b[?1015l".as_slice(),
        b"\x1b[?1003l".as_slice(),
        b"\x1b[?1002l".as_slice(),
        b"\x1b[?1000l".as_slice(),
        b"\x1b[?1049l".as_slice(),
    ];
    let mut cursor = 0;
    for sequence in sequences {
        let relative = output[cursor..]
            .windows(sequence.len())
            .position(|window| window == sequence)
            .unwrap_or_else(|| panic!("missing ordered terminal sequence {sequence:?}"));
        cursor += relative + sequence.len();
    }
}
