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

/// Screen cursor
pub struct Cursor {
    /// Cursor horizontal position.
    pub col: u32,
    /// Cursor vertical position.
    pub row: u32,
    /// First visible line.
    pub top: u8,
    /// Last visible line.
    pub bottom: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum CursorDir {
    /// Right with wrap to the next line.
    Forward(u32),
    /// Left with wrap to the previous line.
    Back(u32),
    /// Beginning of line.
    BOL,
    /// Specific column and row.
    Position(u32, u32)
}
