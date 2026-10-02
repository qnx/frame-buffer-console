/// Escape sequence parser.

use crate::display::DEFAULT_ATTR;

pub struct Escape {
    /// Parser state.
    state: State,
    /// Numeric parameters inside a CSI command.
    params: Vec<u32>,
    /// CSI private prefix.
    prefix: Option<u8>,
    /// Number of characters consumed (0 on parse error).
    pub len: usize,
    /// Parsing result.
    pub result: EscapeResult
}

#[derive(Debug, PartialEq)]
pub enum EscapeResult {
    /// Parsing failed.
    None,
    /// Select Graphic Rendition.
    SGR(u8),
    /// Cursor left.
    CUB(u32),
    /// Cursor down.
    CUD(u32),
    /// Cursor right.
    CUF(u32),
    /// Cursor up.
    CUU(u32),
    /// Cursor position.
    CUP(u32, u32),
    /// Device attributes 2.
    DA2,
    /// Delete character.
    DCH(u32),
    /// Erase in line.
    EL(u32),
    /// Erase in page.
    ED(u32),
    /// Insert character.
    ICH(u32)
}

#[derive(Debug, PartialEq)]
enum State {
    Init,
    CsiParam,
    CsiParamValue,
    CsiIntermediate,
    Done,
    Error
}

impl Escape {
    pub fn new() -> Self {
        Escape { state: State::Init, params: Vec::<u32>::new(), len: 0,
                 result: EscapeResult::None, prefix: None }
    }

    pub fn reset(&mut self) {
        self.state = State::Init;
        self.len = 0;
        self.result = EscapeResult::None;
        self.params.clear();
        self.prefix = None;
    }

    pub fn is_active(&self) -> bool {
        self.state != State::Init
    }

    /// Parse an escape sequence from the given bytes.
    /// The function is intended to be restartable, allowing a resumption of parsing from an
    /// incomplete state given more bytes.
    /// Return true if parsing is complete (either successfully or not), or false if reached the
    /// end of the bytes before an escape sequence terminated.
    pub fn parse(&mut self, data: &[u8]) -> bool {
        let mut param: u32 = 0;
        let mut param_done: bool = false;
        let mut final_char: u8 = 0;
        let mut data_iter = data.into_iter();
        let mut chr_opt = data_iter.next();
        let mut next: bool = false;

        loop {
            let chr = match chr_opt {
                Some(chr) => *chr,
                None => break
            };

            // Escape sequence state machine.
            self.state = match self.state {
                State::Init => {
                    // Look for [ as the beginning of CSI.
                    match chr {
                        b'[' => { next = true; State::CsiParam },
                        _ => State::Error
                    }
                },
                State::CsiParam => {
                    // Parameter, private sequence indicator or final character.
                    if chr >= b'0' && chr <= b'9' {
                        next = false;
                        State::CsiParamValue
                    } else if chr == b'?' || chr == b'>' {
                        self.prefix = Some(chr);
                        next = true;
                        State::CsiParamValue
                    } else if chr >= 0x20 && chr < 0x2f {
                        param_done = true;
                        next = true;
                        State::CsiIntermediate
                    } else if chr >= 0x40 && chr <= 0x7e {
                        param_done = true;
                        final_char = chr;
                        State::Done
                    } else {
                        State::Error
                    }
                }
                State::CsiParamValue => {
                    // Number, parameter separator (;), intermediate or final character.
                    if chr >= b'0' && chr <= b'9' {
                        param = (param * 10) + (chr - b'0') as u32;
                        next = true;
                        State::CsiParamValue
                    } else if chr == b';' {
                        param_done = true;
                        next = true;
                        State::CsiParamValue
                    } else if chr >= 0x20 && chr < 0x2f {
                        param_done = true;
                        next = true;
                        State::CsiIntermediate
                    } else if chr >= 0x40 && chr <= 0x7e {
                        param_done = true;
                        final_char = chr;
                        State::Done
                    } else {
                        State::Error
                    }
                }
                State::CsiIntermediate => {
                    // Another intermediate or final character.
                    if chr >= 0x20 && chr < 0x2f {
                        next = true;
                        State::CsiIntermediate
                    } else if chr >= 0x40 && chr <= 0x7e {
                        final_char = chr;
                        State::Done
                    } else {
                        State::Error
                    }
                }
                _ => State::Error
            };

            // Abort on a parse error.
            if self.state == State::Error {
                let str = bytes_to_string(&data[0..self.len + 1]);
                println!("Escape parse error '{str}'");

                self.len = 0;
                self.result = EscapeResult::None;
                return true;
            }

            if next {
                // Character consumed.
                self.len += 1;
                chr_opt = data_iter.next();
            }

            // Add a parsed numeric parameter.
            if param_done {
                self.params.push(param);
                param = 0;
                param_done = false;
            }

            // Check for end of parsing.
            if self.state == State::Done {
                // Process sequence based on the final character.
                self.result = match final_char {
                    b'm' => self.sgr_to_attr(),
                    b'C' => EscapeResult::CUF(self.param(0, 1)),
                    b'D' => EscapeResult::CUB(self.param(0, 1)),
                    b'P' => EscapeResult::DCH(self.param(0, 1)),
                    b'K' => EscapeResult::EL(self.param(0, 0)),
                    b'J' => EscapeResult::ED(self.param(0, 0)),
                    b'H' => EscapeResult::CUP(self.param(0, 1), self.param(1, 1)),
                    b'@' => EscapeResult::ICH(self.param(0, 1)),
                    b'c' => match self.prefix {
                        Some(b'>') => EscapeResult::DA2,
                        _ => EscapeResult::None
                    }
                    _ => EscapeResult::None
                };

                if self.result == EscapeResult::None {
                    let str = bytes_to_string(&data[0..self.len]);
                    println!("Unhandled escape sequence '{str}'");
                }

                return true;
            }
        }

        false
    }

    fn sgr_to_attr(&self) -> EscapeResult {
        let mut attr: u8 = 0;
        for i in 0..self.params.len() {
            let param = self.params[i];
            if param == 0 {
                // Reset.
                attr = DEFAULT_ATTR;
            } else if param == 7 || param == 27 {
                // Reverse video.
                attr = (attr >> 4) | (attr & 0xf) << 4;
            } else if param >= 30 && param <= 37 {
                // Dark foreground colour.
                attr = (attr & 0xf0) | (param as u8 - 30);
            } else if param >= 40 && param <= 47 {
                // Dark background colour.
                attr = (attr & 0x0f) | ((param as u8 - 40) << 4);
            } else if param >= 90 && param <= 97 {
                // Light foreground colour.
                attr = (attr & 0xf0) | (param as u8 - 82);
            } else if param >= 100 && param <= 107 {
                // Light background colour.
                attr = (attr & 0x0f) | ((param as u8 - 92) << 4);
            }
        }

        EscapeResult::SGR(attr)
    }

    fn param(&mut self, which: usize, default: u32) -> u32 {
        if self.params.len() > which {
            self.params[which]
        } else {
            default
        }
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    let mut str = String::new();
    for chr in bytes {
        str.push(*chr as char);
    }

    str
}
