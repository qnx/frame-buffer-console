use crate::serial::SerialServer;

const DCMD_CHR_TCSETSID: u32 = 0x80040308;
const DCMD_CHR_ISATTY: u32 = 0x318;

/// Safely convert an array of bytes in a message's payload to a structured
/// message.
fn transmute_payload<T>(payload: &[u8]) -> Result<&T, ()> {
    if payload.len() < std::mem::size_of::<T>() {
        // Payload too short for structure.
        return Err(());
    }

    let (head, t, _) = unsafe { payload.align_to::<T>() };
    if !head.is_empty() {
        // Payload not aligned.
        return Err(());
    }

    Ok(&t[0])
}

impl SerialServer {
    /// Top-level devctl dispatcher.
    pub fn devctl(&mut self, dcmd: u32, payload: &[u8]) -> Result<(), i32> {
        println!("devctl {:x}", dcmd);
        match dcmd {
            DCMD_CHR_ISATTY => Ok(()),
            DCMD_CHR_TCSETSID => self.tcsetsid(payload),
            _ => Err(libc::ENOSYS)
        }
    }

    /// Set the session ID for the controlling terminal.
    /// Only one session at a time is allowed to attach to this device.
    fn tcsetsid(&mut self, payload: &[u8]) -> Result<(), i32> {
        let sid = match transmute_payload::<i32>(payload) {
            Ok(sid) => *sid,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        if self.sid.is_some() {
            return Err(libc::EPERM);
        }

        self.sid = Some(sid);
        println!("New session ID {sid}");
        Ok(())
    }
}
