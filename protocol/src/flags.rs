/// From: https://www.sqlite.org/c3ref/c_open_autoproxy.html
pub mod consts {
    pub const SQLITE_OPEN_READONLY: i32 = 0x00000001;
    pub const SQLITE_OPEN_READWRITE: i32 = 0x00000002;
    pub const SQLITE_OPEN_CREATE: i32 = 0x00000004;
    pub const SQLITE_OPEN_DELETEONCLOSE: i32 = 0x00000008;
    pub const SQLITE_OPEN_EXCLUSIVE: i32 = 0x00000010;
    pub const SQLITE_OPEN_AUTOPROXY: i32 = 0x00000020;
    pub const SQLITE_OPEN_URI: i32 = 0x00000040;
    pub const SQLITE_OPEN_MEMORY: i32 = 0x00000080;
    pub const SQLITE_OPEN_MAIN_DB: i32 = 0x00000100;
    pub const SQLITE_OPEN_TEMP_DB: i32 = 0x00000200;
    pub const SQLITE_OPEN_TRANSIENT_DB: i32 = 0x00000400;
    pub const SQLITE_OPEN_MAIN_JOURNAL: i32 = 0x00000800;
    pub const SQLITE_OPEN_TEMP_JOURNAL: i32 = 0x00001000;
    pub const SQLITE_OPEN_SUBJOURNAL: i32 = 0x00002000;
    pub const SQLITE_OPEN_SUPER_JOURNAL: i32 = 0x00004000;
    pub const SQLITE_OPEN_NOMUTEX: i32 = 0x00008000;
    pub const SQLITE_OPEN_FULLMUTEX: i32 = 0x00010000;
    pub const SQLITE_OPEN_SHAREDCACHE: i32 = 0x00020000;
    pub const SQLITE_OPEN_PRIVATECACHE: i32 = 0x00040000;
    pub const SQLITE_OPEN_WAL: i32 = 0x00080000;
    pub const SQLITE_OPEN_NOFOLLOW: i32 = 0x01000000;
    pub const SQLITE_OPEN_EXRESCODE: i32 = 0x02000000;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Flags {
    bits: i32,
}

// From: https://github.com/rusqlite/rusqlite/blob/5c33953b75b59bf93e6ccf391ef884d11c5f4798/src/lib.rs#L1224
impl Default for Flags {
    fn default() -> Self {
        let mut flags = Self::empty();
        flags.set(consts::SQLITE_OPEN_READWRITE, true);
        flags.set(consts::SQLITE_OPEN_CREATE, true);
        flags.set(consts::SQLITE_OPEN_NOMUTEX, true);
        flags.set(consts::SQLITE_OPEN_URI, true);
        flags
    }
}

impl Flags {
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn from_flags(bits: i32) -> Self {
        Self { bits }
    }

    pub const fn bits(&self) -> i32 {
        self.bits
    }

    pub fn set(&mut self, flag: i32, value: bool) {
        if value {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
    }

    pub const fn contains(&self, flag: i32) -> bool {
        (self.bits & flag) == flag
    }
}

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Flags(0b{:b})", self.bits)
    }
}

#[cfg(test)]
mod tests {}
