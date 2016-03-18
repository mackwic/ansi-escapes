#[allow(unused_imports,dead_code)]

use std::convert::From;
use std::default::Default;

#[derive(PartialEq,Eq,Debug)]
pub enum OutputStream<'a> {
    Text(&'a str),
    Control(ControlSeq)
}

#[derive(PartialEq,Eq,Debug)]
pub enum ControlSeq {
    // Esc[H
    // Esc[f
    CursorPositionTopLeft,
    // Esc[Line;ColumnH
    // Esc[Line;Columnf
    CursorPositionYX(usize, usize),
    // Esc[ValueA
    CursorUp(usize),
    // Esc[ValueB
    CursorDown(usize),
    // Esc[ValueC
    CursorForward(usize),
    // Esc[ValueD
    CursorBackward(usize),
    // Esc[s
    SaveCursorPosition,
    // Esc[u
    RestoreCursorPosition,
    // Esc[2J
    EraseDisplay,
    // Esc[K
    EraseLine,
    // Esc[Value;...;Valuem
    SetGraphicMode(Vec<usize>)
    // XXX-NOT IMPLEMENTED:
    // - Esc[=Valueh 	Set Mode:
    //   Changes the screen width or type to the mode specified by one of the following values:
    // - Esc[=Valuel 	Reset Mode:
    //   Resets the mode by using the same values that Set Mode uses, except for 7, which disables
    //   line wrapping
    //   (the last character in this escape sequence is a lowercase L).
    // - Esc[Code;String;...p 	Set Keyboard Strings:
    //   Redefines a keyboard key to a specified string.
}

pub fn parse_seq(input: &[u8], len: usize) -> Option<(usize, ControlSeq)> {
    use ControlSeq::*;
    let mut idx = 0;
    let mut cur_val = String::new();
    let mut values = Vec::new();

    // Second character must always be a '['
    if len == 0 || input[0] as char != '\x5b' {
        return None;
    }

    idx += 1;

    if idx >= len {
        return None;
    }

    match input[idx] as char {
        // Match all one char sequences
        'H' | 'f' => return Some((idx, CursorPositionTopLeft)),
        's' => return Some((idx, SaveCursorPosition)),
        'u' => return Some((idx, RestoreCursorPosition)),
        'K' => return Some((idx, EraseLine)),
        // Try to match 2 char sequence 2J. If not, fallback to a digit in a value
        '2' => {
            if (idx + 1) < len && input[idx+1] as char == 'J' {
                return Some((idx+1, EraseDisplay))
            } else {
                cur_val.push('2')
            }
        },
        // All digits goes to current value
        c@'0'...'9' => cur_val.push(c),
        // Any non-digit or unknown character stop the sequence parsing
        _ => return None
    }
    // If we got there, we just got a digit. It's the only branch not using a return


    // Now the value parsing. We don't know if we have to parse one or many values
    // Anyway we need at least one more char
    if idx + 1 == len {
        return None;
    }

    macro_rules! make_res_cursor {
        ($direction:ident) => {{
            cur_val.parse::<usize>().ok().map(|val| (idx, $direction(val)))
        }}
    }

    macro_rules! parse_val_and_queue {
        () => {{
            let v:usize = match cur_val.parse::<usize>() {
                Err(_) => return None,
                Ok(v) => v
            };
            values.push(v);
            cur_val.clear()
        }}
    }

    while idx < len {
        idx += 1;

        // Parse either 1 value Controls or the first value of the sequence
        match input[idx] as char {
            // add digits in the current value. Only non-breaking branch
            c@'0'...'9' => cur_val.push(c),
            // cursor moves
            'A' => return make_res_cursor!(CursorUp),
            'B' => return make_res_cursor!(CursorDown),
            'C' => return make_res_cursor!(CursorForward),
            'D' => return make_res_cursor!(CursorBackward),
            'm' => {
                return cur_val.parse::<usize>().ok().map(|val| (idx, SetGraphicMode(vec![val])))
            },
            ';' => { parse_val_and_queue!(); break },
            _ => return None
        }
    };

    while idx < len {
        idx += 1;

        // parse sequence of values separated by a ';' ending by [Hfm]

        match input[idx] as char {
            c@'0'...'9' => cur_val.push(c),
            ';' => parse_val_and_queue!(),
            'H'|'f' => {
                if values.len() < 2 {
                    return None;
                } else {
                    // If too many values, simply take the 2 first and carry on
                    let y = values[0];
                    let x = values[1];
                    return Some((idx, CursorPositionYX(y,x)))
                }
            },
            'm' => {
                parse_val_and_queue!();
                return Some((idx, SetGraphicMode(values)))
            }
            _ => return None
        }
    }

    None
}

#[cfg(test)]
mod tests_parseseq {
    use super::*;

    macro_rules! test_simple_escape {
        ($name:ident: $str_esc:expr => $control:expr) => {
            #[test]
            fn $name() {
                let b = $str_esc.as_bytes();
                assert_eq!(parse_seq(b, b.len()), Some((b.len() - 1, $control)))
            }
        }
    }

    macro_rules! test_none {
        ($name:ident: $str_esc:expr) => {
            #[test]
            fn $name() {
                let b = $str_esc.as_bytes();
                assert_eq!(parse_seq(b, b.len()), None)
            }
        }
    }

    test_simple_escape!(save_cursor: "[s" => ControlSeq::SaveCursorPosition);
    test_simple_escape!(restore_cursor: "[u" => ControlSeq::RestoreCursorPosition);
    test_simple_escape!(cursor_top_left1: "[H" => ControlSeq::CursorPositionTopLeft);
    test_simple_escape!(cursor_top_left2: "[f" => ControlSeq::CursorPositionTopLeft);
    test_simple_escape!(erase_display: "[2J" => ControlSeq::EraseDisplay);
    test_simple_escape!(erase_line: "[k" => ControlSeq::EraseLine);
    test_simple_escape!(cursor_pos_1: "[0;222H" => ControlSeq::CursorPositionYX(0,222));
    test_simple_escape!(cursor_pos_2: "[22;19H" => ControlSeq::CursorPositionYX(22,19));
    test_simple_escape!(cursor_up: "[9999A" => ControlSeq::CursorUp(9999));
    test_simple_escape!(cursor_down: "[000000B" => ControlSeq::CursorDown(0));
    test_simple_escape!(cursor_forward: "[1234567890C" => ControlSeq::CursorForward(1234567890));
    test_simple_escape!(cursor_backward: "[1D" => ControlSeq::CursorBackward(1));

    test_none!(too_short1: "[");
    test_none!(too_short2: "");
    test_none!(too_short3: ")");
    test_none!(too_short4: " [s");

    test_none!(bad_format1: "[1s");
    test_none!(bad_format2: "[1u");
    test_none!(bad_format3: "[1;1s");
    test_none!(bad_format4: "[1k");
    test_none!(bad_format5: "[1;1D");
    test_none!(bad_format6: "[m");
    test_none!(bad_format7: "[J");
    test_none!(bad_format8: "[;m");
    test_none!(bad_format82: "[;;;;m");
    test_none!(bad_format9: "[;H");
    test_none!(bad_format10: "[;f");
    test_none!(bad_format11: "[1");
    test_none!(bad_format12: "[11234231321312");
    test_none!(bad_format13: "[112342313;");
    test_none!(bad_format14: "[112342313;");
    test_none!(bad_format15: "[1.1A");
}


pub fn parse_str(input: &str) -> Vec<OutputStream> {
    if input.len() == 0 {
        return vec![]
    }

    let mut res : Vec<OutputStream> = Vec::default();
    let bytes = input.as_bytes();
    let mut idx = 0;
    let mut watch = 0;
    let len = bytes.len();
    let mut first_loop = true;

    while idx < len {

        // as the idx is an uint, we don't want to increment it the first time
        if first_loop {
            first_loop = false
        } else {
            idx += 1;
        }

        // look for the escape character
        if bytes[idx] as char != '\x1b' {
            continue
        }

        // push text only if we saw text
        if idx != watch {
            unsafe {
                res.push(OutputStream::Text(std::str::from_utf8_unchecked(&bytes[watch..(idx-1)])));
            }
        }


        // found the escape char, try to parse an ANSI control sequence
        match parse_seq(&bytes[idx+1..], (len - idx)) {
            None => continue,
            Some((seqsize, control_seq)) => {
                res.push(OutputStream::Control(control_seq));

                idx += seqsize + 1;
                watch = idx
            }
        }
    }

    if idx != watch {
        unsafe {
            res.push(OutputStream::Text(std::str::from_utf8_unchecked(&bytes[idx..])))
        }
    }

    return res;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(parse_str("").len(), 0);
        //assert_eq!(parse_str("toto").as_slice(), &[ColoredString::from("toto")])
    }
}
