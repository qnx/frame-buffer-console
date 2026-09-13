mod framebuffer;
mod libc_missing;

use framebuffer::FrameBuffer;

pub struct Vga {
}

impl Vga {
    pub fn new() -> Result<Vga, ()> {
        let fb = FrameBuffer::new()?;
        println!("{:?}", fb);
        Ok(Vga {})
    }
}
