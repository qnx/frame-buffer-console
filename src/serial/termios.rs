const DEFAULT_IFLAG: u32 = libc::IMAXBEL | libc::INPCK | libc::BRKINT | libc::ICRNL | libc::IXON |
                        libc::IXOFF;
const DEFAULT_OFLAG: u32 = libc::ONLCR | libc::OPOST;
const DEFAULT_CFLAG: u32 = libc::CREAD | libc::CS8 | libc::HUPCL | libc::CLOCAL;
const DEFAULT_LFLAG: u32 = libc::ECHO | libc::ECHOCTL | libc::ECHOE | libc::ECHOK | libc::ECHONL |
                        libc::ECHOKE | libc::ICANON | libc::ISIG | libc::IEXTEN;

pub struct TerminalAttributes {
    pub input: u32,
    pub output: u32,
    pub control: u32,
    pub local: u32
}

impl TerminalAttributes {
    pub fn new() -> Self {
        TerminalAttributes {
            input: DEFAULT_IFLAG,
            output: DEFAULT_OFLAG,
            control: DEFAULT_CFLAG,
            local: DEFAULT_LFLAG
        }
    }

    pub fn as_termios(&self) -> libc::termios {
        let mut tios: libc::termios = unsafe { std::mem::zeroed() };
        tios.c_iflag = self.input;
        tios.c_oflag = self.output;
        tios.c_cflag = self.control;
        tios.c_lflag = self.local;
        tios.c_ispeed = 100000;
        tios.c_ospeed = 100000;
        tios
    }

    pub fn echo_on(&self) -> bool {
        (self.local & libc::ECHO) != 0
    }
}
