mod x86_64;

use std::io::{Read, Write};

#[cfg(target_arch = "x86_64")]
pub(crate) use self::x86_64::*;
