pub use kr_core::*;

#[cfg(feature = "macros")]
pub use kr_macros::*;

use std::panic::Location;

#[track_caller]
pub fn make_ctx(msg: impl Into<String>) -> String {
    let loc = Location::caller();
    format!("{} ({}:{})", msg.into(), loc.file(), loc.line())
}
