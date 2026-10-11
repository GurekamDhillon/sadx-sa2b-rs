//! Game knowledge for Sonic Adventure 2 and Sonic Adventure DX, built on
//! `sa_formats`: install detection, physics tables, enums, the character
//! asset catalog and the Chao save format.

pub mod anim;
pub mod catalog;
pub mod chao_save;
pub mod enums;
pub mod error;
pub mod install;
pub mod physics;
pub mod rig;

pub use error::{Error, Result};
pub use install::{Game, GameKind};
pub use rig::Rig;
