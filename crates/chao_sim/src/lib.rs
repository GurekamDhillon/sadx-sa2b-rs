//! Chao raising simulation for SA2-format Chao (`sa_data::chao_save::Chao`).
//!
//! Every function reads and writes the game's own save fields, so a Chao
//! raised here can be written back to `SONIC2B__ALF` and vice versa.
//!
//! Numbers come from two kinds of source, kept apart on purpose:
//! - **Verified** constants (from Chao Island's SA2 / modern-version pages,
//!   cited at each use) are plain `const`s or literals.
//! - **Unverified** numbers live in [`Tuning`] with documented defaults, so
//!   they can be corrected without touching the logic.
//!
//! The game's own Chao code was not decompiled: SA2's exe is DRM-encrypted.

pub mod items;
pub mod life;
pub mod rng;
pub mod stats;
pub mod tuning;

pub use items::{apply_animal, apply_drive, apply_fruit, care, ChaosDrive, Fruit};
pub use life::{age, cocoon_outcome, evolution_type, evolve, CocoonOutcome, LifeEvent};
pub use rng::Rng;
pub use stats::{add_progress, level_gain, points_cap};
pub use tuning::Tuning;
