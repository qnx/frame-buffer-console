// Copyright (c) 2026 BlackBerry Limited.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::display::DEFAULT_ATTR;

/// Escape sequence parser.
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
    /// ASCII character set.
    ASCII,
    /// Cursor character absolute.
    CHA(u32),
    /// Cursor left.
    CUB(u32),
    /// Cursor down.
    CUD(u32),
    /// Cursor right.
    CUF(u32),
    /// Cursor position.
    CUP(u32, u32),
    /// Cursor up.
    CUU(u32),
    /// Device attributes 2.
    DA2,
    /// Delete character.
    DCH(u32),
    /// Application cursor keys.
    DECCKMA,
    /// Normal cursor keys.
    DECCKMN,
    /// Application keypad.
    DECPAM,
    /// Normal keypad.
    DECPNM,
    /// DEC Special Graphics character set.
    DECSG,
    /// Erase in line.
    EL(u32),
    /// Erase in page.
    ED(u32),
    /// Insert character.
    ICH(u32),
    /// Insert mode.
    IRMI,
    /// Replace mode.
    IRMR,
    /// Select Graphic Rendition.
    SGR(u8),
    /// Line position absolute.
    VPA(u32)
}

#[derive(Debug, PartialEq)]
enum State {
    Init,
    CsiParam,
    CsiParamValue,
    CsiIntermediate,
    CsiFinal,
    String,
    StringTerminator,
    CharSet,
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
    pub fn parse(&mut self, data: &[u8], attr: u8) -> bool {
        let mut param: u32 = 0;
        let mut param_done: bool = false;
        let mut data_iter = data.into_iter();
        let mut chr_opt = data_iter.next();

        loop {
            let chr = match chr_opt {
                Some(chr) => *chr,
                None => break
            };

            // true to consume the character, false to repeat it after changing state.
            let mut next: bool = true;

            // Escape sequence state machine.
            self.state = match self.state {
                State::Init => {
                    match chr {
                        b'M' => State::Done,     // Reverse index
                        b'P' => State::String,   // Device Control String
                        b'[' => State::CsiParam, // CSI
                        b']' => State::String,   // Operating System Command
                        b'^' => State::String,   // Privacy Message
                        b'_' => State::String,   // Application Program Command
                        b'=' => {
                            self.result = EscapeResult::DECPAM;
                            State::Done
                        },                       // Application keypad
                        b'>' => {
                            self.result = EscapeResult::DECPNM;
                            State::Done
                        },                       // Normal keypad
                        b'(' => State::CharSet,  // Character set
                        b')' => State::CharSet,  // Character set
                        b'*' => State::CharSet,  // Character set
                        b'+' => State::CharSet,  // Character set
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
                        State::CsiParamValue
                    } else if chr >= 0x20 && chr < 0x2f {
                        State::CsiIntermediate
                    } else if chr >= 0x40 && chr <= 0x7e {
                        next = false;
                        State::CsiFinal
                    } else {
                        State::Error
                    }
                }
                State::CsiParamValue => {
                    // Number, parameter separator (;), intermediate or final character.
                    if chr >= b'0' && chr <= b'9' {
                        param = (param * 10) + (chr - b'0') as u32;
                        State::CsiParamValue
                    } else if chr == b';' {
                        param_done = true;
                        State::CsiParamValue
                    } else if chr >= 0x20 && chr < 0x2f {
                        param_done = true;
                        State::CsiIntermediate
                    } else if chr >= 0x40 && chr <= 0x7e {
                        param_done = true;
                        next = false;
                        State::CsiFinal
                    } else {
                        State::Error
                    }
                }
                State::CsiIntermediate => {
                    // Another intermediate or final character.
                    if chr >= 0x20 && chr < 0x2f {
                        State::CsiIntermediate
                    } else if chr >= 0x40 && chr <= 0x7e {
                        next = false;
                        State::CsiFinal
                    } else {
                        State::Error
                    }
                }
                State::CsiFinal => {
                    self.csi(chr, attr);
                    State::Done
                }
                State::String => {
                    if chr >= 0x20 && chr <= 0x7e {
                        State::String
                    } else if chr >= 0x08 && chr <= 0x0d {
                        State::String
                    } else if chr == 0x1b {
                        State::StringTerminator
                    } else if chr == 0x7 {
                        State::Done     // xterm string terminator
                    } else {
                        State::Error
                    }
                }
                State::StringTerminator => {
                    match chr {
                        b'\\' => State::Done,
                        _ => State::Error
                    }
                }
                State::CharSet => {
                    // Character sets other than ASCII and DEC special graphics are parsed but
                    // ignored.
                    if chr == b'0' {
                        self.result = EscapeResult::DECSG;
                        State::Done
                    } else if chr == b'B' {
                        self.result = EscapeResult::ASCII;
                        State::Done
                    } else if chr >= b'A' && chr <= b'Z' {
                        State::Done
                    } else if chr >= b'0' && chr <= b'9' {
                        State::Done
                    } else if chr == b'=' {
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
                println!("Escape parse error {:?} '{str}'", data);

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
                self.debug_unhandled(data);
                return true;
            }
        }

        false
    }

    /// Terminate a command sequence introducer.
    /// The final character determines the resulting command.
    fn csi(&mut self, final_char: u8, attr: u8) {
        // Process sequence based on the final character.
        self.result = match final_char {
            b'm' => self.sgr_to_attr(attr),
            b'A' => EscapeResult::CUU(self.param(0, 1)),
            b'B' => EscapeResult::CUD(self.param(0, 1)),
            b'C' => EscapeResult::CUF(self.param(0, 1)),
            b'D' => EscapeResult::CUB(self.param(0, 1)),
            b'G' => EscapeResult::CHA(self.param(0, 1)),
            b'H' => EscapeResult::CUP(self.param(0, 1), self.param(1, 1)),
            b'J' => EscapeResult::ED(self.param(0, 0)),
            b'K' => EscapeResult::EL(self.param(0, 0)),
            b'P' => EscapeResult::DCH(self.param(0, 1)),
            b'@' => EscapeResult::ICH(self.param(0, 1)),
            b'c' => match self.prefix {
                Some(b'>') => EscapeResult::DA2,
                _ => EscapeResult::None
            }
            b'd' => EscapeResult::VPA(self.param(0, 1)),
            b'h' => if self.prefix == Some(b'?') {
                EscapeResult::DECCKMA
            } else {
                match self.param(0, 0) {
                    4 => EscapeResult::IRMI,
                    _ => EscapeResult::None
                }
            }
            b'l' => if self.prefix == Some(b'?') {
                EscapeResult::DECCKMN
            } else {
                match self.param(0, 0) {
                    4 => EscapeResult::IRMR,
                    _ => EscapeResult::None
                }
            }
            _ => EscapeResult::None
        };
    }

    /// Convert a CSI graphics command to character attributes.
    /// Currently only handles foreground and background colours.
    fn sgr_to_attr(&self, mut attr: u8) -> EscapeResult {
        for i in 0..self.params.len() {
            let param = self.params[i];
            if param == 0 {
                // Reset.
                attr = DEFAULT_ATTR;
            } else if param == 1 {
                // Bold - use the bright version of the foreground colour.
                attr |= 0x08;
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

    fn debug_unhandled(&self, _data: &[u8]) {
        #[cfg(debug_assertions)]
        {
            if self.result == EscapeResult::None {
                let str = bytes_to_string(&_data[0..self.len]);
                println!("Unhandled escape sequence '{str}'");
            }
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
