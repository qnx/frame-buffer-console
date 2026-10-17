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
use crate::display::escape::*;
use crate::display::cursor::*;
use crate::serial::SerialServer;

/// The logical contents of the screen
pub struct Screen {
    /// Screen contents (character and attrbitute for each position).
    /// The two elements are for the primary (0) and alternative (1) buffers.
    buffers: [ScreenBuffer; 2],
    /// Which buffer is active.
    curbuf: usize,
    /// Horizontal size in characters.
    pub columns: u32,
    /// Vertical size in characters.
    pub rows: u32,
    /// Cursor position and shape.
    pub cursor: Cursor,
    /// Region of updated contents.
    dirty: Option<Rect>,
    /// Current attributes.
    attr: u8,
    /// Escape sequence parser.
    pub escape: Escape,
    /// Whether DEC Special Grpahics character set is in effect.
    decsg: bool,
    /// Top line of the scrolling region.
    top_scroll: u32,
    /// Bottom line of the scrolling region (inclusive).
    bottom_scroll: u32,
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
    pub fn new(columns: u32, rows: u32, cursor: Cursor) -> Result<Screen, ()> {
        println!("Screen size {columns}x{rows}");
        Ok(Screen {
            buffers: [ ScreenBuffer::new(columns, rows), ScreenBuffer::new(columns, rows) ],
            curbuf: 0,
            columns,
            rows,
            dirty: Some(Rect { left: 0, top: 0, right: columns - 1, bottom: rows - 1 }),
            cursor,
            attr: DEFAULT_ATTR,
            escape: Escape::new(),
            decsg: false,
            top_scroll: 0,
            bottom_scroll: rows - 1
        })
    }

    /// Get the character at the given column and row.
    pub fn char_at(&self, col: u32, row: u32) -> ScreenChar {
        let idx = ((row * self.columns) + col) as usize;
        self.buffers[self.curbuf].get_char(idx)
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
                b'\r' => self.move_cursor(CursorDir::BOL, &mut insert),
                _ => self.insert_char(chr, &mut insert)
            }

            pos += 1;
        }

        // Set new cursor position.
        insert.update_rect(self.cursor.col, self.cursor.row);

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
                insert.update_rect(self.cursor.col, self.cursor.row);
                self.newline(&mut insert);
                break;
            }

            // The line may exceed the width of the screen. Drop extra characters, but keep
            // processing to detect a newline.
            if self.cursor.col < self.columns {
                self.buffers[self.curbuf].set_char(insert.bufidx,
                                                   ScreenChar { chr, attr: self.attr });
                insert.bufidx += 1;
                self.cursor.col += 1;
            }
        }

        // Set new cursor position.
        insert.update_rect(self.cursor.col, self.cursor.row);
        self.cursor.col = cursor_col;

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);
    }

    pub fn move_in_line(&mut self, dir: CursorDir) {
        // Add characters.
        let mut insert = Insert::new(self, self.cursor.col, self.cursor.row);
        self.move_cursor(dir, &mut insert);

        // Set new cursor position.
        insert.update_rect(self.cursor.col, self.cursor.row);

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);
    }

    fn insert_char(&mut self, mut chr: u8, insert: &mut Insert) {
        if self.decsg {
            // Translate special graphics characters.
            if chr >= 0x5f && chr <= 0x7e {
                chr = DECSG_TABLE[(chr - 0x5f) as usize];
            }
        }

        // Update the buffer with the given character.
        self.buffers[self.curbuf].set_char(insert.bufidx, ScreenChar { chr, attr: self.attr });
        insert.bufidx += 1;

        // Update bounding rectangle of this insertion.
        insert.update_rect(self.cursor.col, self.cursor.row);

        // Move the position one to the right, wrapping as necessary.
        if self.cursor.col == self.columns - 1 {
            self.update_cursor(0, self.cursor.row + 1, insert);
        } else {
            self.cursor.col += 1;
        }

    }

    /// Move the cursor to the beginning of the next line.
    fn newline(&mut self, insert: &mut Insert) {
        self.update_cursor(0, self.cursor.row + 1, insert);
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
            EscapeResult::ALTBUF(on) => self.altbuf(on),
            EscapeResult::ASCII => self.decsg = false,
            EscapeResult::CHA(c) =>
                self.move_cursor(CursorDir::Position(c, self.cursor.row + 1), insert),
            EscapeResult::CUB(n) => self.move_cursor(CursorDir::Back(n), insert),
            EscapeResult::CUF(n) => self.move_cursor(CursorDir::Forward(n), insert),
            EscapeResult::CUP(r, c) => self.move_cursor(CursorDir::Position(c, r), insert),
            EscapeResult::DCH(n) => self.delete(n, insert),
            EscapeResult::DECCKMA => SerialServer::set_cursor_key_escape_char(b'O'),
            EscapeResult::DECCKMN => SerialServer::set_cursor_key_escape_char(b'['),
            EscapeResult::DECSG => self.decsg = true,
            EscapeResult::DECSTBM(t, b) => self.set_scroll_region(t, b, insert),
            EscapeResult::EL(n) => self.erase_line(n, insert),
            EscapeResult::ED(n) => self.erase_page(n, insert),
            EscapeResult::ICH(n) => self.shift(n, insert),
            EscapeResult::IL(n) => self.insert_lines(n, insert),
            EscapeResult::RI => self.reverse_index(insert),
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
    fn move_cursor(&mut self, dir: CursorDir, insert: &mut Insert) {
        // FIXME:
        // Handle the num argument.
        let (mut col, mut row) = (self.cursor.col, self.cursor.row);
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

        // Do not call update_cursor(): these movements should not cause scrolling.
        self.cursor.col = col;
        self.cursor.row = row;
        insert.bufidx = (self.cursor.row * self.columns + self.cursor.col) as usize;
    }

    /// Delete num characters from the current position.
    fn delete(&mut self, num: u32, insert: &mut Insert) {
        for i in 0..num as usize {
            if (insert.bufidx + i) == self.buffers[self.curbuf].size() {
                break;
            }

            self.buffers[self.curbuf].set_char(insert.bufidx + i, BLANK_CHAR);
        }
    }

    /// Delete characters in the display, according to the semantics of the Erase Line (EL) control
    /// sequence:
    /// 0 - from cursor position to end of line
    /// 1 - from beginning of line to cursor position (inclusive)
    /// 2 - all
    fn erase_line(&mut self, how: u32, insert: &mut Insert) {
        let (first_idx, count) = match how {
            0 => (insert.bufidx, self.columns - self.cursor.col),
            1 => (insert.bufidx - self.cursor.col as usize, self.cursor.col + 1),
            2 => (insert.bufidx - self.cursor.col as usize, self.columns),
            _ => { return; }
        };

        for idx in first_idx..(first_idx + count as usize) {
            self.buffers[self.curbuf].set_char(idx, BLANK_CHAR);
        }

        // Assume the entire line needs to be updated.
        insert.update_rect(0, self.cursor.row);
        insert.update_rect(self.columns - 1, self.cursor.row);
    }

    /// Delete characters in the display, according to the semantics of the Erase Page (ED) control
    /// sequence:
    /// 0 - from cursor position to end of display
    /// 1 - from beginning of display to cursor position (inclusive)
    /// 2 - all
    fn erase_page(&mut self, how: u32, insert: &mut Insert) {
        let (first_idx, last_idx) = match how {
            0 => (insert.bufidx, self.buffers[self.curbuf].size() - 1),
            1 => (0, insert.bufidx),
            2 => (0, self.buffers[self.curbuf].size() - 1),
            _ => { return; }
        };

        for idx in first_idx..=last_idx {
            self.buffers[self.curbuf].set_char(idx, BLANK_CHAR);
        }

        // Assume the entire screen needs to be updated.
        insert.rect = self.full_rect();
    }

    /// Shift characters in a line to the right when inserting a character.
    fn shift(&mut self, num: u32, insert: &mut Insert) {
        let (col, row) = (self.cursor.col, self.cursor.row);
        let num = if num > self.columns - col {
            self.columns - col
        } else {
            num
        } as usize;

        // Move characters in the line.
        let eolidx = ((row + 1) * self.columns) as usize - num;
        for idx in (insert.bufidx..eolidx).rev() {
            self.buffers[self.curbuf].set_char(idx + num, self.buffers[self.curbuf].get_char(idx));
        }
        for idx in 0..num {
            self.buffers[self.curbuf].set_char(insert.bufidx + idx, BLANK_CHAR);
        }

        // Mark the line as dirty.
        insert.update_rect(self.columns - 1, row);
    }

    /// Set new columns and row values for the cursor.
    /// The function should be called whenever the cursor's row may change, in order to detect and
    /// apply scrolling. If it is known that the row doesn't change, then the cursor's column can
    /// be updated directly.
    fn update_cursor(&mut self, col: u32, row: u32, insert: &mut Insert) {
        // Determine if scrolling is needed.
        if row > self.bottom_scroll && self.cursor.row <= self.bottom_scroll {
            // Rotate the buffer.
            self.buffers[self.curbuf].scroll_up(self.top_scroll, self.bottom_scroll);

            // Now inserting into the new bottom row.
            self.cursor.row = self.bottom_scroll;

            // The entire screen needs to be updated.
            self.dirty = Some(self.full_rect());
        } else if row >= self.rows {
            // Shouldn't happen, but if a new line is opened beneath the last row, and the scroll
            // window is smaller than the entire buffer, overwrite the last row.
            self.cursor.row = self.rows - 1;
        } else {
            self.cursor.row = row;
        }

        self.cursor.col = col;
        insert.bufidx = (self.cursor.row * self.columns + self.cursor.col) as usize;
    }

    /// Return a rectangle encompassing the entire screen
    fn full_rect(&self) -> Rect {
        Rect { left: 0, top: 0, right: self.columns - 1, bottom: self.rows - 1 }
    }

    /// Select between the primary and alternative buffers.
    fn altbuf(&mut self, on: bool) {
        // Choose the right buffer.
        self.curbuf = match on {
            true => 1,
            false => 0
        };

        // Redraw the entire screen.
        self.set_dirty(&self.full_rect());

        // Reset the scroll area.
        // FIXME:
        // It's not clear how the scroll area is supposed to be restored, so doing it here just
        // ensure that exiting a program like Vim does not leave a restricted scroll window behind.
        self.top_scroll = 0;
        self.bottom_scroll = self.rows - 1;
    }

    /// Set a new scroll region.
    /// Row values are 1-based.
    fn set_scroll_region(&mut self, top: u32, bottom: u32, insert: &mut Insert) {
        self.bottom_scroll = {
            if bottom > self.rows {
                self.rows - 1
            } else if bottom > 0 {
                bottom - 1
            } else {
                self.rows - 1
            }
        };

        self.top_scroll = {
            if top > self.bottom_scroll {
                0
            } else if top > 0 {
                top - 1
            } else {
                0
            }
        };

        self.cursor.col = 0;
        self.cursor.row = self.top_scroll;
        insert.bufidx = (self.cursor.row * self.columns + self.cursor.col) as usize;
    }

    /// Move the cursor one line up, scrolling if the cursor is already at the top margin.
    fn reverse_index(&mut self, insert: &mut Insert) {
        if self.cursor.row == self.top_scroll {
            self.buffers[self.curbuf].scroll_down(self.top_scroll, self.bottom_scroll);
            insert.rect = self.full_rect();
        } else {
            self.move_cursor(CursorDir::Position(self.cursor.col + 1, self.cursor.row),
                             insert);
        }
    }

    /// Insert blank lines.
    fn insert_lines(&mut self, n: u32, insert: &mut Insert) {
        // Has no effect outside the scroll region.
        let row = self.cursor.row;
        if row < self.top_scroll || row > self.bottom_scroll {
            return;
        }


        // FIXME:
        // scroll_down() could do multiple lines at a time, avoiding excessive copies, but is this
        // a common enough case to optimize?
        for _ in 0..n {
            self.buffers[self.curbuf].scroll_down(row, self.bottom_scroll);
        }

        insert.rect = self.full_rect();
    }
}

/// Character and attribute buffer with scrolling.
struct ScreenBuffer {
    /// The buffer
    data: Vec<ScreenChar>,
    /// Number of characters in a line
    stride: usize,
}

impl ScreenBuffer {
    /// Create a new screen buffer for the given size.
    pub fn new(columns: u32, rows: u32) -> Self {
        let size = (columns * rows) as usize;
        let mut data = Vec::<ScreenChar>::with_capacity(size);
        data.resize(size, BLANK_CHAR);

        ScreenBuffer {
            data,
            stride: columns as usize
        }
    }

    /// Retreive the character at the given index.
    pub fn get_char(&self, index: usize) -> ScreenChar {
        self.data[index]
    }

    /// Update the character at the given index.
    pub fn set_char(&mut self, index: usize, chr: ScreenChar) {
        self.data[index] = chr;
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Move the contents of the scroll window up one line, discarding the top line.
    pub fn scroll_up(&mut self, top_row: u32, bottom_row: u32) {
        // Move the contents of [top_row + 1,bottom_row) into [top_row, bottom_row - 1).
        let dst_idx = top_row as usize * self.stride;
        let src_idx = dst_idx + self.stride;
        let len = (bottom_row - top_row) as usize * self.stride;
        unsafe { std::ptr::copy(&self.data[src_idx], &mut self.data[dst_idx], len) };

        // Clear the bottom row.
        let bottom_idx = bottom_row as usize * self.stride;
        for i in 0..self.stride {
            self.data[bottom_idx + i] = BLANK_CHAR;
        }
    }

    /// Move the contents of the scroll window down one line, discarding the bottom line.
    pub fn scroll_down(&mut self, top_row: u32, bottom_row: u32) {
        // Move the contents of [top_row, bottom_row - 1) into [top_row + 1, bottom_row).
        let src_idx = top_row as usize * self.stride;
        let dst_idx = src_idx + self.stride;
        let len = (bottom_row - top_row) as usize * self.stride;
        unsafe { std::ptr::copy(&self.data[src_idx], &mut self.data[dst_idx], len) };

        // Clear the top row.
        let top_idx = top_row as usize * self.stride;
        for i in 0..self.stride {
            self.data[top_idx + i] = BLANK_CHAR;
        }
    }
}

/// Tracks single- and multi-characters insertions into the screen's buffer.
struct Insert {
    /// Position in the screen buffer.
    bufidx: usize,
    /// Area of the screen affected by the update.
    rect: Rect,
}

impl Insert {
    pub fn new(screen: &Screen, col: u32, row: u32) -> Self {
        Insert {
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
