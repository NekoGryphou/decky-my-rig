// ! Terminal echo through termios, for reading the pairing code without showing it.
use myrig_client::platform::InputEcho;
use std::mem::MaybeUninit;

/// Echo is off until this is dropped, which restores the settings it found.
pub(super) struct Echo {
    fd: i32,
    original: libc::termios,
}

impl Echo {
    /// Turns echo off on standard input. `None` when it isn't a terminal, where
    /// there is no echo to hide.
    pub(super) fn hide() -> Option<Self> {
        Self::hide_on(libc::STDIN_FILENO)
    }

    /// Turns echo off on `fd`, which must be a terminal to hide anything.
    pub(super) fn hide_on(fd: i32) -> Option<Self> {
        // ! SAFETY: isatty only reads the descriptor's type.
        if unsafe { libc::isatty(fd) } != 1 {
            return None;
        }
        let mut current = MaybeUninit::<libc::termios>::uninit();
        // ! SAFETY: tcgetattr writes the settings only when it returns 0, which is checked before
        // ! they are read.
        if unsafe { libc::tcgetattr(fd, current.as_mut_ptr()) } != 0 {
            return None;
        }
        // ! SAFETY: tcgetattr initialized the structure, as the check above showed.
        let original = unsafe { current.assume_init() };
        let mut hidden = original;
        hidden.c_lflag &= !libc::ECHO;
        // ! SAFETY: `hidden` is a complete termios copied from this terminal.
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &hidden) } != 0 {
            return None;
        }
        Some(Self { fd, original })
    }
}

impl Drop for Echo {
    fn drop(&mut self) {
        // ! SAFETY: `original` came from tcgetattr on the same descriptor. A failure here leaves
        // ! echo off, which nothing can report from drop, so it is ignored.
        unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.original) };
    }
}

impl InputEcho for Echo {}
