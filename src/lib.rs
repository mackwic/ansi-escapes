#[allow(unused_imports,dead_code)]

use std::convert::From;
use std::default::Default;

pub enum OutputStream<'a> {
    Text(&'a str),
    Control(ControlSeq)
}

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

#[derive(PartialEq,Eq,Debug)]
pub enum Color { Black }
#[derive(PartialEq,Eq,Debug)]
pub struct Style;

#[derive(PartialEq,Eq,Debug)]
pub struct ColoredString {
    input: String,
    fgcolor: Option<Color>,
    bgcolor: Option<Color>,
    style: Option<Style>
}

impl Default for ColoredString {
    fn default() -> ColoredString {
        ColoredString {
            input: String::new(),
            fgcolor: None,
            bgcolor: None,
            style: None
        }
    }
}

impl<'a> From<&'a str> for ColoredString {
    fn from(input: &'a str) -> Self {
        ColoredString { input: String::from(input), .. ColoredString::default() }
    }
}

//#[derive(Debug,Default)]
//struct ParserState;

fn parse_seq(input: &[u8], len: usize) -> Option<(usize, ControlSeq)> {
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

        match input[idx] as char {
            // add digits in the current value
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
        assert_eq!(parse_str("toto").as_slice(), &[ColoredString::from("toto")])
    }
}
