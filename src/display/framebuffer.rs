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

/// Access to the hardware frame buffer
pub struct FrameBuffer {
    /// Virtual address of the mapped hardware frame buffer.
    hw_addr: MemoryMap<u32>,
    /// Virtual address of a software buffer used for double-buffering.
    sw_addr: MemoryMap<u32>,
    /// Size of the buffer, in pixels.
    size: usize,
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

#[cfg(target_arch = "x86_64")]
const FB_CACHE_ATTR: u32 = libc_missing::X86_64_PTE_MTYPE_WC as u32;

// FIXME:
// May not need strongly-ordered device memory.
#[cfg(target_arch = "aarch64")]
const FB_CACHE_ATTR: u32 = libc_missing::AARCH64_PTE_SODEV as u32;

/// Initialize the buffers used for screen display.
/// The hardware frame buffer is mapped using a shared memory object, while the software buffer is
/// allocated in regular memory.
fn map_framebuffer(paddr: u64, len: usize, double_buffer: bool) ->
    Result<(MemoryMap<u32>, MemoryMap<u32>), ()> {
    // Create the shared memory object.
    let fd = unsafe { libc::shm_open(libc::SHM_ANON, libc::O_RDWR, 0o600) };
    if fd == -1 {
        println!("Failed to create shared memory object");
        return Err(());
    }

    // Populate the object with the physical range of the hardware buffer.
    let rc = unsafe {
        libc_missing::shm_ctl_special(fd, libc_missing::SHMCTL_PHYS, paddr, len as u64,
                                      FB_CACHE_ATTR)
    };

    if rc == -1 {
        println!("Failed to populate shared memory object");
        return Err(());
    }

    // Map the hardware buffer.
    let hw_addr = MemoryMap::map_fd(fd, len, libc::PROT_READ | libc::PROT_WRITE)?;

    // Allocate a software buffer.
    let sw_addr = if double_buffer {
        MemoryMap::map_anon(len, libc::PROT_READ | libc::PROT_WRITE)?
    } else {
        hw_addr
    };

    Ok((hw_addr, sw_addr))
}

impl FrameBuffer {
    /// Map and allocate a frame buffer.
    pub fn new(double_buffer: bool) -> Result<FrameBuffer, ()> {
        let hfb = hwi_framebuffer()?;
        let size = hfb.bufsize as usize;
        let (hw_addr, sw_addr) = map_framebuffer(hfb.buffer, size, double_buffer)?;

        Ok(FrameBuffer {
            hw_addr,
            sw_addr,
            size: size / size_of::<u32>(),
            width: hfb.hres,
            height: hfb.vres,
            stride: hfb.stride / size_of::<u32>() as u32
        })
    }

    /// Write a value into the software buffer.
    /// A call to update() is required to copy the contents to the hardware buffer.
    pub fn set_pixel(&self, pixel: u64, value: u32) {
        assert!(pixel < self.size as u64);
        unsafe { *self.sw_addr.addr.wrapping_add(pixel as usize) = value };
    }

    /// Copy the contents of the software buffer to the mapped hardware buffer.
    /// Only applicable if double-buffering is enabled.
    pub fn update(&self) {
        if self.sw_addr.addr != self.hw_addr.addr {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.sw_addr.addr,
                    self.hw_addr.addr,
                    self.size)
            };
        }
    }
}

/// Memory obtained via a mmap() call.
/// The memory can be shared safely among all threads in the process.
#[derive(Clone, Copy)]
struct MemoryMap<T> {
    addr: *mut T
}

impl<T> MemoryMap<T> {
    /// Allocate anonymous private memory from the system.
    pub fn map_anon(size: usize, prot: i32) -> Result<Self, ()> {
        let addr = unsafe {
            libc::mmap(std::ptr::null_mut(), size,
                       prot, libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0)
        };

        if addr == libc::MAP_FAILED {
            return Err(());
        }

        Ok(MemoryMap { addr: addr as *mut T })
    }

    /// Memory-map a file descriptor using a shared mapping.
    pub fn map_fd(fd: i32, size: usize, prot: i32) -> Result<Self, ()> {
        let addr = unsafe {
            libc::mmap(std::ptr::null_mut(), size, prot, libc::MAP_SHARED, fd, 0)
        };

        if addr == libc::MAP_FAILED {
            return Err(());
        }

        Ok(MemoryMap { addr: addr as *mut T })
    }
}

unsafe impl<T> Send for MemoryMap<T> {
}
