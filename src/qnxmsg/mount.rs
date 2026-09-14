use std::ffi::CString;
use std::ffi::CStr;
use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;

/// Mount point attachement object.
pub struct Mount
{
    mount_handle: u32
}

impl Mount
{
    /// Create a new instance of a message server.
    pub fn new() -> Self {
        Mount {
            mount_handle: 0
        }
    }

    /// Attach to a path.
    pub fn attach(&mut self, channel: &Channel, path: &str, file_type: FileType,
                  flags: u32)
                  -> Result<(i32, String), i32> {
        // Register with path manager.
        let attach_path = match CString::new(path) {
            Ok(s) => s,
            Err(_) => { return Err(libc::EINVAL) }
        };

        self.mount_handle += 1;
        let fd = match pathmgr_link(&attach_path, channel, self.mount_handle,
                                    file_type, flags) {
            Ok(fd) => fd,
            Err(err) => { return Err(err); }
        };

        // Add the node to the table, using /mount_handle as the path.
        // The use of a leading / should prevent conflicts with other nodes.
        Ok((fd, format!("/{}", self.mount_handle)))
    }
}

fn get_errno() -> i32 {
    unsafe {
        *libc::__get_errno_ptr()
    }
}

mod syscalls {
    unsafe extern "C" {
        pub fn pathmgr_link(path: *const libc::c_char, nd: i32,
                            pid: libc::pid_t, chid: i32, handle: u32,
                            file_type: u32, flags: u32) -> i32;
    }
}

fn pathmgr_link(path: &CStr, channel: &Channel, handle: u32, file_type: FileType,
                flags: u32) -> Result<i32, i32> {
    let fd = unsafe {
        let path = path.as_ptr() as *const libc::c_char;
        let pid = libc::getpid();
        let chid = channel.chid;
        let file_type = file_type.value();
        syscalls::pathmgr_link(path, 0, pid, chid, handle, file_type, flags)
    };

    if fd == -1 {
        return Err(get_errno());
    }

    Ok(fd)
}
