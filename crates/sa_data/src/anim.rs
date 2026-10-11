//! Per-character animation tables (`AnimationInfo[]`) from `sonic2app.exe`.
//!
//! Each character has a list indexed by animation state (the
//! `CHARACTER_ANIMATIONS` enum of the X-Hax headers: 0 stand, 6 walk, 8 jog,
//! 9 run, 65 jump, ...). An entry names the motion to play (`anim_num`, an
//! id in `plcommtn.prs` for 0..=85 or the character's own `*mtn.prs`), the
//! model to draw it on, how to play it and which state follows.

use sa_formats::bytes::{AddressMap, Bytes, Endian};
use sa_formats::pe::Pe;

use crate::enums::Character;
use crate::error::{Error, Result};

pub const ENTRY_SIZE: usize = 16;

/// `AnimationInfo` (16 bytes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimInfo {
    /// Motion id.
    pub anim_num: i16,
    /// Index of the model in the character's model file (not the id).
    pub model_num: i16,
    /// Playback mode: 0 = hold the last frame, 2 = loop (SA2 `mtnmode`).
    pub mtn_mode: i16,
    /// State that follows when this one ends.
    pub next: i16,
    /// Blend speed into this state.
    pub transition_speed: f32,
    /// Frames advanced per game frame (1.0 = normal; scaled by speed for run cycles).
    pub anim_speed: f32,
}

/// (table address, entry count) in the SA2 Steam exe.
pub fn sa2_table(c: Character) -> Option<(u32, usize)> {
    use Character::*;
    Some(match c {
        Sonic | SuperSonic => (0xA17C18, 203),
        Shadow | SuperShadow => (0xA188C8, 249),
        Tails => (0xA0D608, 118),
        Eggman => (0xA12750, 164),
        Knuckles => (0xA13320, 217),
        Rouge => (0xA150F0, 217),
        MechTails => (0xA0EAA0, 190),
        MechEggman => (0xA0E1F0, 139),
        Amy => (0xA19858, 249),
        MetalSonic => (0xA1A7E8, 249),
        ChaoWalker => (0xA0F680, 198),
        DarkChaoWalker => (0xA102E0, 202),
        Tikal => (0xA140B0, 260),
        Chaos => (0xA15E80, 272),
        B => return None,
    })
}

pub fn read_table(exe: &[u8], address: u32, count: usize) -> Result<Vec<AnimInfo>> {
    let pe = Pe::parse(exe)?;
    let off = pe.offset(address).ok_or_else(|| Error::NotFound(format!("animation table {address:#x} not mapped in exe")))?;
    let b = Bytes::new(exe, Endian::Little);
    (0..count)
        .map(|i| {
            let o = off + i * ENTRY_SIZE;
            Ok(AnimInfo { anim_num: b.i16(o)?, model_num: b.i16(o + 2)?, mtn_mode: b.i16(o + 4)?, next: b.i16(o + 6)?, transition_speed: b.f32(o + 8)?, anim_speed: b.f32(o + 12)? })
        })
        .collect()
}

/// Animation state numbers used by Sonic-style characters (`CHARACTER_ANIMATIONS`).
pub mod state {
    pub const STAND: usize = 0;
    pub const WALK: usize = 6;
    pub const SPEEDWALK: usize = 7;
    pub const JOG: usize = 8;
    pub const RUN: usize = 9;
    pub const SPRINT: usize = 10;
    pub const SPINDASH: usize = 11;
    pub const SPINDASH_1: usize = 12;
    pub const BRAKE: usize = 18;
    pub const DAMAGE: usize = 21;
    pub const BALL_SPIN: usize = 30;
    pub const JUMP: usize = 65;
    pub const JUMP_SPIN: usize = 66;
    pub const JUMP_BEGINFALL: usize = 67;
    pub const JUMP_FALL: usize = 68;
    pub const LAND_LOW: usize = 16;
    pub const SONIC_HOMING_LUNGE: usize = 100;
    pub const SONIC_ATTACK: usize = 118;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_character_has_a_table_but_b() {
        for c in Character::ALL {
            assert_eq!(sa2_table(c).is_some(), c != Character::B, "{c:?}");
        }
    }

    #[test]
    fn decodes_entries() {
        // Build a fake PE with one section and two entries.
        let mut payload = Vec::new();
        for (n, m, mode, next, tr, sp) in [(0i16, 1i16, 2i16, 0i16, 0.25f32, 1.0f32), (6, 1, 2, 6, 0.125, 0.5)] {
            for v in [n, m, mode, next] {
                payload.extend_from_slice(&v.to_le_bytes());
            }
            payload.extend_from_slice(&tr.to_le_bytes());
            payload.extend_from_slice(&sp.to_le_bytes());
        }
        let exe = sa_formats::pe::tests_support::tiny_pe(&payload);
        let t = read_table(&exe, 0x401000, 2).unwrap();
        assert_eq!(t[1], AnimInfo { anim_num: 6, model_num: 1, mtn_mode: 2, next: 6, transition_speed: 0.125, anim_speed: 0.5 });
        assert!(read_table(&exe, 0x300000, 1).is_err());
    }
}
