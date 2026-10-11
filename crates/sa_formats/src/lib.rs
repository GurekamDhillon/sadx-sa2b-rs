//! File formats used by Sonic Adventure 2 and Sonic Adventure DX.
//!
//! Pure parsing: no knowledge of which file holds what.

pub mod bytes;
pub mod error;
pub mod ninja;
pub mod pak;
pub mod pe;
pub mod prs;
pub mod texture;

pub use error::{Error, Result};
