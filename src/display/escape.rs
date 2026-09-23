/// Escape sequence parser.

pub struct Escape {
    /// Parser state.
    state: State,
    /// Numeric parameters inside a CSI command.
    params: Vec<u32>,
    /// Number of characters consumed (0 on parse error).
    pub len: usize,
    /// Parsing result.
    pub result: EscapeResult
}

#[derive(Debug)]
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
    /// Delete character.
    DCH(u32),
    /// Erase in line.
    EL(u32)
}

#[derive(Debug, PartialEq)]
enum State {
    Init,
    CsiParameter,
    CsiIntermediate,
    Done,
    Error
}

impl Escape {
    pub fn new() -> Self {
        Escape { state: State::Init, params: Vec::<u32>::new(), len: 0,
                 result: EscapeResult::None }
    }

    pub fn reset(&mut self) {
        self.state = State::Init;
        self.len = 0;
        self.result = EscapeResult::None;
        self.params.clear();
    }

    pub fn is_active(&self) -> bool {
        self.state != State::Init
    }

    pub fn parse(&mut self, mut iter: std::slice::Iter<'_, u8>) {
        let mut param: u32 = 0;
        let mut param_done: bool = false;
        let mut final_char: u8 = 0;
        loop {
            // Get the next character, aborting if reached the end before the Done state.
            let chr = match iter.next() {
                Some(chr) => *chr,
                None => { break; }
            };

            // Escape sequence state machine.
            self.state = match self.state {
                State::Init => {
                    match chr {
                        b'[' => State::CsiParameter,
                        _ => State::Error
                    }
                },
                State::CsiParameter => {
                    if chr >= b'0' && chr <= b'9' {
                        param = (param * 10) + (chr - b'0') as u32;
                        State::CsiParameter
                    } else if chr == b';' {
                        param_done = true;
                        State::CsiParameter
                    } else if chr == b'?' {
                        // Private sequence?
                        State::CsiParameter
                    } else if chr >= 0x20 && chr < 0x2f {
                        param_done = true;
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
                    if chr >= 0x20 && chr < 0x2f {
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
                self.len = 0;
                break;
            }

            // Character consumed.
            self.len += 1;

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
                    b'C' => EscapeResult::CUF(self.param(1)),
                    b'D' => EscapeResult::CUB(self.param(1)),
                    b'P' => EscapeResult::DCH(self.param(1)),
                    b'K' => EscapeResult::EL(self.param(1)),
                    _ => EscapeResult::None
                };

                return;
            }
        }
    }

    fn sgr_to_attr(&self) -> EscapeResult {
        let mut attr: u8 = 0;
        for i in 0..self.params.len() {
            let param = self.params[i];
            if param == 0 {
                // Reset.
                attr = 0;
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

    fn param(&mut self, default: u32) -> u32 {
        if self.params.len() > 0 {
            self.params[0]
        } else {
            default
        }
    }
}
