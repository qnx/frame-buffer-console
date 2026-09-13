mod framebuffer;
mod screen;
mod font;
mod libc_missing;

use framebuffer::FrameBuffer;
use screen::Screen;

pub struct Vga {
}

impl Vga {
    pub fn new() -> Result<Vga, ()> {
        // Map the frame buffer.
        let framebuffer = FrameBuffer::new()?;

        // Construct the screen object.
        let screen = Screen::new(framebuffer.width, framebuffer.height, &font::FONT_8X8)?;
        Ok(Vga {})
    }
}
