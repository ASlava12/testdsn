use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    Interrupt,
    Terminate,
}

impl ShutdownSignal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interrupt => "sigint",
            Self::Terminate => "sigterm",
        }
    }
}

static SIGNAL_STATE: AtomicU8 = AtomicU8::new(0);

pub fn install_shutdown_handlers() -> Result<(), String> {
    #[cfg(unix)]
    {
        unix::install_shutdown_handlers()
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

pub fn pending_shutdown_signal() -> Option<ShutdownSignal> {
    match SIGNAL_STATE.load(Ordering::SeqCst) {
        1 => Some(ShutdownSignal::Interrupt),
        2 => Some(ShutdownSignal::Terminate),
        _ => None,
    }
}

pub fn process_exists(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unix::process_exists(pid)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

#[cfg(unix)]
mod unix {
    use std::{io, os::raw::c_int, sync::atomic::Ordering};

    use super::SIGNAL_STATE;

    const SIGINT: c_int = 2;
    const SIGTERM: c_int = 15;
    const SIG_ERR: usize = usize::MAX;

    type SignalHandler = usize;

    unsafe extern "C" {
        fn kill(pid: c_int, signal: c_int) -> c_int;
        fn signal(signal: c_int, handler: SignalHandler) -> SignalHandler;
    }

    pub(super) fn install_shutdown_handlers() -> Result<(), String> {
        unsafe {
            if signal(SIGINT, handle_shutdown_signal as *const () as SignalHandler) == SIG_ERR {
                return Err(format!(
                    "failed to install SIGINT handler: {}",
                    io::Error::last_os_error()
                ));
            }
            if signal(
                SIGTERM,
                handle_shutdown_signal as *const () as SignalHandler,
            ) == SIG_ERR
            {
                return Err(format!(
                    "failed to install SIGTERM handler: {}",
                    io::Error::last_os_error()
                ));
            }
        }
        Ok(())
    }

    pub(super) fn process_exists(pid: u32) -> bool {
        if pid == 0 || pid > c_int::MAX as u32 {
            return false;
        }

        let result = unsafe { kill(pid as c_int, 0) };
        if result == 0 {
            return true;
        }

        !matches!(io::Error::last_os_error().raw_os_error(), Some(3))
    }

    extern "C" fn handle_shutdown_signal(signal: c_int) {
        let code = match signal {
            SIGINT => 1,
            SIGTERM => 2,
            _ => 0,
        };
        if code != 0 {
            let _ = SIGNAL_STATE.compare_exchange(0, code, Ordering::SeqCst, Ordering::SeqCst);
        }
    }
}
