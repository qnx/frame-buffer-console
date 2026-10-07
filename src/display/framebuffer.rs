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

use crate::libc_missing;

/// Access to the UEFI frame buffer
#[derive(Debug)]
pub struct FrameBuffer {
    /// Virtual address of the mapped hardware frame buffer.
    pub hw_addr: u64,
    /// Virtual address of a software buffer used for double-buffering.
    pub sw_addr: u64,
    /// Size of the buffer, in bytes.
    pub size: u32,
    /// Horizontial resolution in pixels.
    pub width: u32,
    /// Vertical resolution in pixels.
    pub height: u32,
    /// Number of pixels per row.
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
fn map_framebuffer(paddr: u64, len: usize) -> Result<(u64, u64), ()> {
    let hw_addr = unsafe {
            libc::mmap(std::ptr::null_mut(), len,
                       libc::PROT_READ | libc::PROT_WRITE | libc_missing::PROT_NOCACHE,
                       libc::MAP_SHARED | libc_missing::MAP_PHYS, -1, paddr as i64)
    };

    if hw_addr == libc::MAP_FAILED {
        println!("Failed to map frame buffer");
        return Err(());
    }

    let sw_addr = unsafe {
            libc::mmap(std::ptr::null_mut(), len,
                       libc::PROT_READ | libc::PROT_WRITE,
                       libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0)
    };

    if sw_addr == libc::MAP_FAILED {
        println!("Failed to map frame buffer");
        return Err(());
    }

    Ok((hw_addr as u64, sw_addr as u64))
}

impl FrameBuffer {
    pub fn new() -> Result<FrameBuffer, ()> {
        let hfb = hwi_framebuffer()?;
        let (hw_addr, sw_addr) = map_framebuffer(hfb.buffer, hfb.bufsize as usize)?;
        Ok(FrameBuffer { hw_addr, sw_addr, size: hfb.bufsize, width: hfb.hres, height: hfb.vres,
                         stride: hfb.stride })
    }

    /// Write a value into the frame buffer.
    pub fn set_pixel(&self, pixel: u64, value: u32) {
        assert!(pixel < self.size as u64);
        unsafe { *((self.sw_addr + pixel) as *mut u32) = value };
    }

    pub fn update(&self) {
        unsafe {
            libc::memcpy(self.hw_addr as *mut libc::c_void, self.sw_addr as *const libc::c_void,
                         self.size as usize);
        };
    }
}
