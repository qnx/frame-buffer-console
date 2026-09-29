/// Access to the UEFI frame buffer

use crate::libc_missing;

#[derive(Debug)]
pub struct FrameBuffer {
    pub addr: u64,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct HwiFrameBuffer {
    size: u16,
    name: u16,
    pad: u32,
    buffer: u64,
    bufsize: u32,
    hres: u32,
    vres: u32,
    stride: u32
}

/// Find the HWI tag for the frame buffer.
/// The tag holds the physical address of the buffer, as well as its dimensions.
fn hwi_framebuffer() -> Result<HwiFrameBuffer, ()> {
    let off = unsafe {
        libc_missing::hwi_find_tag(libc_missing::HWI_NULL_OFF as u32, 0,
                                   c"framebuffer".as_ptr() as *const libc::c_char)
    };

    if off == libc_missing::HWI_NULL_OFF as u32 {
        println!("Failed to find framebuffer tag");
        return Err(());
    }

    let hfb = unsafe {
        let ptr = libc_missing::hwi_off2tag(off);
        *(ptr as *const HwiFrameBuffer)
    };

    Ok(hfb)
}

/// Map the physical address of the frame buffer.
fn map_framebuffer(paddr: u64, len: usize) -> Result<u64, ()> {
    let vaddr = unsafe {
            libc::mmap(std::ptr::null_mut(), len,
                       libc::PROT_READ | libc::PROT_WRITE | libc_missing::PROT_NOCACHE,
                       libc::MAP_SHARED | libc_missing::MAP_PHYS, -1, paddr as i64)
    };

    if vaddr == libc::MAP_FAILED {
        println!("Failed to map frame buffer");
        return Err(());
    }

    Ok(vaddr as u64)
}

impl FrameBuffer {
    pub fn new() -> Result<FrameBuffer, ()> {
        let hfb = hwi_framebuffer()?;
        let addr = map_framebuffer(hfb.buffer, hfb.bufsize as usize)?;
        Ok(FrameBuffer { addr, size: hfb.bufsize, width: hfb.hres, height: hfb.vres,
                         stride: hfb.stride })
    }

    /// Write a value into the frame buffer.
    pub fn set_pixel(&self, pixel: u64, value: u32) {
        assert!(pixel < self.size as u64);
        unsafe { *((self.addr + pixel) as *mut u32) = value };
    }
}
