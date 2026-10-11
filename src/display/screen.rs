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

use crate::display::*;
use crate::display::font::Font;
use crate::display::escape::*;
use crate::display::cursor::*;

/// The logical contents of the screen
pub struct Screen {
    /// Screen contents (character and attrbitute for each position).
    buffer: ScreenBuffer,
    /// Horizontal size in characters.
    pub columns: u32,
    /// Vertical size in characters.
    pub rows: u32,
    /// Cursor position and shape.
    pub cursor: Cursor,
    /// Translate carriage return to newline.
    pub cr_to_nl: bool,
    /// Region of updated contents.
    dirty: Option<Rect>,
    /// Current attributes.
    attr: u8,
    /// Escape sequence parser.
    pub escape: Escape,
    /// Whether DEC Special Grpahics character set is in effect.
    decsg: bool
}

const BLANK_CHAR: ScreenChar = ScreenChar {
    chr: ' ' as u8,
    attr: DEFAULT_ATTR
};

const DECSG_TABLE: [u8; 32] = [
    0x20, 0x4, 0xdb, 0x0, 0x0, 0x0, 0x0, 0xf8, 0xf1, 0xb2, 0xe8, 0xd9, 0xbf, 0xda, 0xc0, 0xc5,
    0xdf, 0xdf, 0xc4, 0xdc, 0xdc, 0xc3, 0xb4, 0xc1, 0xc2, 0xb3, 0xf3, 0xf2, 0xe3, 0x0, 0x0, 0x7
];

impl Screen {
    pub fn new(width: u32, height: u32, font: &Font) -> Result<Screen, ()> {
        // Calculate the number of columns and rows.
        let columns = width / font.width as u32;
        let rows = height / font.height as u32;
        Ok(Screen {
            buffer: ScreenBuffer::new(columns, rows),
            columns,
            rows,
            dirty: Some(Rect { left: 0, top: 0, right: columns - 1, bottom: rows - 1 }),
            cursor: Cursor { col: 0, row: 0, top: font.height - 1, bottom: font.height - 1 },
            attr: DEFAULT_ATTR,
            escape: Escape::new(),
            cr_to_nl: false,
            decsg: false
        })
    }

    /// Get the character at the given column and row.
    pub fn char_at(&self, col: u32, row: u32) -> ScreenChar {
        let idx = ((row * self.columns) + col) as usize;
        self.buffer.get_char(idx)
    }

    /// Get the current dirty rectangle and reset to an empty one.
    pub fn get_rect(&mut self) -> Option<Rect> {
        let rect = self.dirty;
        self.dirty = None;
        rect
    }

    /// Mark a region as dirty.
    /// The region is combined with the existing dirty rectangle.
    fn set_dirty(&mut self, rect: &Rect) -> bool {
        let mut is_dirty = false;
        let mut dirty = match self.dirty {
            Some(mut dirty) => {
                // Update existing rectangle.
                if rect.left < dirty.left {
                    dirty.left = rect.left;
                    is_dirty = true;
                }

                if rect.right > dirty.right {
                    dirty.right = rect.right;
                    is_dirty = true;
                }

                if rect.top < dirty.top {
                    dirty.top = rect.top;
                    is_dirty = true;
                }

                if rect.bottom > dirty.bottom {
                    dirty.bottom = rect.bottom;
                    is_dirty = true;
                }

                dirty
            }
            None => {
                // Accept new rectangle.
                is_dirty = true;
                *rect
            }
        };

        if is_dirty {
            // Clip to screen size.
            dirty.right = std::cmp::min(dirty.right, self.columns - 1);
            dirty.bottom = std::cmp::min(dirty.bottom, self.rows - 1);
            self.dirty = Some(dirty);
        }

        is_dirty
    }

    /// Insert multiple characters at the cursor's position.
    /// The cursor is updated to one past the last inserted character's position.
    pub fn insert_at_cursor(&mut self, chrs: &[u8]) {
        // Mark starting position.
        let mut insert = Insert::new(self, self.cursor.col, self.cursor.row);
        let mut pos = 0;

        // Resume parsing an escape sequence from a previous call.
        // FIXME:
        // This doesn't really handle all cases of resuming from a previous call.
        // 1. An incomplete parameter
        // 2. A sequence that ends in a parse error doesn't remember the accumulated characters
        if self.escape.is_active() {
            pos += self.maybe_escape(&mut insert, chrs);
        }

        // Insert characters.
        while pos < chrs.len() {
            let chr = chrs[pos];
            match chr {
                b'\n' => self.newline(&mut insert),
                0x07 => self.bell(),
                0x08 => self.move_cursor(CursorDir::Back(1), &mut insert),
                0x1b => pos += self.maybe_escape(&mut insert, &chrs[pos + 1..]),
                b'\r' => {
                    if self.cr_to_nl {
                        self.newline(&mut insert)
                    } else {
                        self.move_cursor(CursorDir::BOL, &mut insert)
                    }
                }
                _ => self.insert_char(chr, &mut insert)
            }

            pos += 1;
        }

        // Set new cursor position.
        self.check_scroll(&mut insert);
        self.cursor.col = insert.col;
        self.cursor.row = insert.row;
        insert.update_rect(insert.col, insert.row);

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);
    }

    /// Insert characters at the given row and column.
    /// This is used to implement canonical mode line editing, and therefore should only affect the
    /// given row.
    /// Note that it is assumed all characters are displayable (should be taken care of by the line
    /// editor).
    pub fn insert_line(&mut self, chrs: &[u8], row: u32, col: u32, cursor_col: u32) {
        // Add characters.
        let mut insert = Insert::new(self, col, row);

        // Insert characters.
        for chr in chrs {
            let chr = *chr;

            // Stop at a newline.
            if chr == b'\n' {
                // Add the current position to the dirty rectangle, as it may hold a cursor that
                // needs to be cleared.
                insert.update_rect(insert.col, insert.row);
                self.newline(&mut insert);
                break;
            }

            // The line may exceed the width of the screen. Drop extra characters, but keep
            // processing to detect a newline.
            if insert.col < self.columns {
                self.buffer.set_char(insert.bufidx, ScreenChar { chr, attr: self.attr });
                insert.bufidx += 1;
                insert.col += 1;
            }
        }

        // Set new cursor position.
        self.check_scroll(&mut insert);
        self.cursor.col = cursor_col;
        self.cursor.row = insert.row;
        insert.update_rect(insert.col, insert.row);

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);
    }

    pub fn move_in_line(&mut self, dir: CursorDir) {
        // Add characters.
        let mut insert = Insert::new(self, self.cursor.col, self.cursor.row);
        self.move_cursor(dir, &mut insert);

        // Set new cursor position.
        self.check_scroll(&mut insert);
        self.cursor.col = insert.col;
        self.cursor.row = insert.row;
        insert.update_rect(insert.col, insert.row);

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);
    }

    fn insert_char(&mut self, mut chr: u8, insert: &mut Insert) {
        // Check for scrolling if inserting into an overflow row.
        self.check_scroll(insert);

        if self.decsg {
            // Translate special graphics characters.
            if chr >= 0x5f && chr <= 0x7e {
                chr = DECSG_TABLE[(chr - 0x5f) as usize];
            }
        }

        // Update the buffer with the given character.
        self.buffer.set_char(insert.bufidx, ScreenChar { chr, attr: self.attr });
        insert.bufidx += 1;

        // Update bounding rectangle of this insertion.
        insert.update_rect(insert.col, insert.row);

        // Move the position one to the right, wrapping as necessary.
        insert.col += 1;
        if insert.col == self.columns {
            insert.col = 0;
            insert.row += 1;
        }
    }

    /// Move the cursor to the beginning of the next line.
    fn newline(&mut self, insert: &mut Insert) {
        // Check for scrolling if inserting into an overflow row.
        self.check_scroll(insert);

        // Move to the next line.
        insert.col = 0;
        insert.row += 1;
        insert.bufidx = (insert.row * self.columns) as usize;
    }

    fn bell(&mut self) {
        // Do nothing for now.
    }

    /// Handle a potential escape sequence.
    /// The function is called when the ESC character is encouneted in a stream.
    fn maybe_escape(&mut self, insert: &mut Insert, data: &[u8]) -> usize {
        // Parse the escape sequence.
        if !self.escape.parse(data, self.attr) {
            return self.escape.len;
        }

        // Act on the result.
        match self.escape.result {
            EscapeResult::ASCII => self.decsg = false,
            EscapeResult::CHA(c) =>
                self.move_cursor(CursorDir::Position(c, self.cursor.row + 1), insert),
            EscapeResult::CUB(n) => self.move_cursor(CursorDir::Back(n), insert),
            EscapeResult::CUF(n) => self.move_cursor(CursorDir::Forward(n), insert),
            EscapeResult::CUP(r, c) => self.move_cursor(CursorDir::Position(c, r), insert),
            EscapeResult::DCH(n) => self.delete(n, insert),
            EscapeResult::DECSG => self.decsg = true,
            EscapeResult::EL(n) => self.erase_line(n, insert),
            EscapeResult::ED(n) => self.erase_page(n, insert),
            EscapeResult::ICH(n) => self.shift(n, insert),
            EscapeResult::SGR(attr) => self.attr = attr,
            EscapeResult::VPA(r) =>
                self.move_cursor(CursorDir::Position(self.cursor.col + 1, r), insert),
            _ => {}
        }

        let len = self.escape.len;
        self.escape.reset();
        len
    }

    /// Update the insertion position based on the given direction.
    fn move_cursor(&self, dir: CursorDir, insert: &mut Insert) {
        // FIXME:
        // Handle the num argument.
        let (mut col, mut row) = (insert.col, insert.row);
        match dir {
            CursorDir::Forward(n) => {
                // FIXME:
                // No support for n beyond EOL. Does it matter?
                if col + n < self.columns {
                    // Within the same row.
                    col += n;
                } else {
                    // Move to the beginning of the next row.
                    // May require scrolling.
                    col = 0;
                    row += 1;
                }
            }
            CursorDir::Back(n) => {
                if col >= n {
                    col -= n;
                } else if row > 0 {
                    col = self.columns - 1;
                    row -= 1;
                } else {
                    col = 0;
                    row = 0;
                }
            }
            CursorDir::BOL => {
                col = 0;
            }
            CursorDir::Position(c, r) => {
                col = {
                    if c >= self.columns {
                        self.columns - 1
                    } else if c > 0 {
                        c - 1
                    } else {
                        0
                    }
                };
                row = {
                    if r >= self.rows {
                        self.rows - 1
                    } else if r > 0 {
                        r - 1
                    } else {
                        0
                    }
                };
            }
        };

        insert.col = col;
        insert.row = row;
        insert.bufidx = ((row * self.columns) + col) as usize;
    }

    /// Delete num characters from the current position.
    fn delete(&mut self, num: u32, insert: &mut Insert) {
        for i in 0..num as usize {
            if (insert.bufidx + i) == self.buffer.size() {
                break;
            }

            self.buffer.set_char(insert.bufidx + i, BLANK_CHAR);
        }
    }

    /// Delete characters in the display, according to the semantics of the Erase Line (EL) control
    /// sequence:
    /// 0 - from cursor position to end of line
    /// 1 - from beginning of line to cursor position (inclusive)
    /// 2 - all
    fn erase_line(&mut self, how: u32, insert: &mut Insert) {
        self.check_scroll(insert);

        let (first_idx, count) = match how {
            0 => (insert.bufidx, self.columns - insert.col),
            1 => (insert.bufidx - insert.col as usize, insert.col + 1),
            2 => (insert.bufidx - insert.col as usize, self.columns),
            _ => { return; }
        };

        for idx in first_idx..(first_idx + count as usize) {
            self.buffer.set_char(idx, BLANK_CHAR);
        }

        // Assume the entire line needs to be updated.
        insert.update_rect(0, insert.row);
        insert.update_rect(self.columns - 1, insert.row);
    }

    /// Delete characters in the display, according to the semantics of the Erase Page (ED) control
    /// sequence:
    /// 0 - from cursor position to end of display
    /// 1 - from beginning of display to cursor position (inclusive)
    /// 2 - all
    fn erase_page(&mut self, how: u32, insert: &mut Insert) {
        self.check_scroll(insert);

        let (first_idx, last_idx) = match how {
            0 => (insert.bufidx, self.buffer.size() - 1),
            1 => (0, insert.bufidx),
            2 => (0, self.buffer.size() - 1),
            _ => { return; }
        };

        for idx in first_idx..=last_idx {
            self.buffer.set_char(idx, BLANK_CHAR);
        }

        // Assume the entire screen needs to be updated.
        insert.rect = self.full_rect();
    }

    /// Shift characters in a line to the right when inserting a character.
    fn shift(&mut self, num: u32, insert: &mut Insert) {
        let (col, row) = (insert.col, insert.row);
        let num = if num > self.columns - col {
            self.columns - col
        } else {
            num
        } as usize;

        // Move characters in the line.
        let eolidx = ((row + 1) * self.columns) as usize - num;
        for idx in (insert.bufidx..eolidx).rev() {
            self.buffer.set_char(idx + num, self.buffer.get_char(idx));
        }
        for idx in 0..num {
            self.buffer.set_char(insert.bufidx + idx, BLANK_CHAR);
        }

        // Mark the line as dirty.
        insert.update_rect(self.columns - 1, row);
    }

    fn check_scroll(&mut self, insert: &mut Insert) {
        // Check if scolling is needed.
        if insert.row < self.rows {
            return;
        }

        // Rotate the buffer.
        self.buffer.scroll();

        // Now inserting into the new bottom row.
        insert.row -= 1;
        insert.bufidx = (insert.row * self.columns) as usize;

        // The entire screen needs to be updated.
        self.dirty = Some(self.full_rect());
    }

    /// Return a rectangle encompassing the entire screen
    fn full_rect(&self) -> Rect {
        Rect { left: 0, top: 0, right: self.columns - 1, bottom: self.rows - 1 }
    }
}

/// Character and attribute buffer with scrolling.
struct ScreenBuffer {
    /// The buffer
    data: Vec<ScreenChar>,
    /// Number of characters in a line
    stride: usize,
    /// Index of the top-left position in the buffer
    head: usize
}

impl ScreenBuffer {
    /// Create a new screen buffer for the given size.
    pub fn new(columns: u32, rows: u32) -> Self {
        let size = (columns * rows) as usize;
        let mut data = Vec::<ScreenChar>::with_capacity(size);
        data.resize(size, BLANK_CHAR);

        ScreenBuffer {
            data,
            head: 0,
            stride: columns as usize
        }
    }

    /// Retreive the character at the given index.
    pub fn get_char(&self, index: usize) -> ScreenChar {
        let index = (self.head + index) % self.data.len();
        self.data[index]
    }

    /// Update the character at the given index.
    pub fn set_char(&mut self, index: usize, chr: ScreenChar) {
        let index = (self.head + index) % self.data.len();
        self.data[index] = chr;
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Scroll one line by adjusting the head position and clearing the line that is now at the
    /// bottom of the screen.
    pub fn scroll(&mut self) {
        self.head = (self.head + self.stride) % self.data.len();
        for i in 0..self.stride {
            self.data[self.head + i] = BLANK_CHAR;
        }
    }
}

/// Tracks single- and multi-characters insertions into the screen's buffer.
struct Insert {
    /// Column for the next character.
    col: u32,
    /// Row for the next character.
    row: u32,
    /// Position in the screen buffer.
    bufidx: usize,
    /// Area of the screen affected by the update.
    rect: Rect,
}

impl Insert {
    pub fn new(screen: &Screen, col: u32, row: u32) -> Self {
        Insert {
            col,
            row,
            bufidx: (row * screen.columns + col) as usize,
            rect: Rect { left: col, right: col, top: row, bottom: row }
        }
    }

    pub fn update_rect(&mut self, col: u32, row: u32) {
        if col < self.rect.left {
            // Can happen on a new line.
            self.rect.left = col;
        }

        if col > self.rect.right {
            self.rect.right = col;
        }

        if row < self.rect.top {
            self.rect.top = row;
        }

        if row > self.rect.bottom {
            self.rect.bottom = row;
        }
    }
}
