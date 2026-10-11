//! SA2 Chao data (`ChaoData`, 0x800 bytes per slot, little-endian on PC).
//!
//! Each slot keeps its raw bytes; typed accessors read and write fields in
//! place, so bytes nobody understands survive a read/write round trip.
//! Offsets are from `ChaoDataBase` in the X-Hax `SA2Structs.h`.
//!
//! Unverified: where the slot array starts inside `SONIC2B__ALF`. In memory
//! `ChaoSlots` (0x19F6EC4) is 0xA64 bytes after `ChaoSaveStart` (0x19F6460);
//! the file is assumed to be a dump from `ChaoSaveStart`, so slots start at
//! [`SA2_SLOT_OFFSET`]. Pass another offset to [`Sa2ChaoFile::read_at`] if a
//! real save shows otherwise.

use crate::enums::{Animal, ChaoStat, ChaoType};
use crate::error::{Error, Result};

pub const SLOT_SIZE: usize = 0x800;
pub const SA2_SLOT_COUNT: usize = 37;
pub const SA2_SLOT_OFFSET: usize = 0x19F6EC4 - 0x19F6460;

/// Field offsets inside one slot.
pub mod off {
    pub const NAME: usize = 0x12;
    pub const NAME_LEN: usize = 7;
    pub const STAT_FRACTIONS: usize = 0x20;
    pub const STAT_GRADES: usize = 0x28;
    pub const STAT_LEVELS: usize = 0x30;
    pub const STAT_POINTS: usize = 0x38;
    pub const TYPE: usize = 0x80;
    pub const GARDEN: usize = 0x81;
    pub const HAPPINESS: usize = 0x82;
    pub const IN_KINDERGARTEN: usize = 0x84;
    pub const CLOCK_ROLLOVERS: usize = 0x86;
    pub const ADULT_CLOCK_ROLLOVERS: usize = 0x88;
    pub const LIFESPAN: usize = 0x8A;
    pub const LIFESPAN2: usize = 0x8C;
    pub const REINCARNATIONS: usize = 0x8E;
    pub const SEED: usize = 0x98;
    pub const TIMESCALE_TIMER: usize = 0xA4;
    pub const POWER_RUN: usize = 0xA8;
    pub const FLY_SWIM: usize = 0xAC;
    pub const ALIGNMENT: usize = 0xB0;
    pub const LAST_LIFE_ALIGNMENT: usize = 0xBC;
    pub const EVOLUTION_PROGRESS: usize = 0xC0;
    pub const EYE_TYPE: usize = 0xD1;
    pub const MOUTH_TYPE: usize = 0xD2;
    pub const BALL_TYPE: usize = 0xD3;
    pub const HEADGEAR: usize = 0xD5;
    pub const HIDE_FEET: usize = 0xD6;
    pub const MEDAL: usize = 0xD7;
    pub const COLOR: usize = 0xD8;
    pub const MONOTONE: usize = 0xD9;
    pub const TEXTURE: usize = 0xDA;
    pub const SHINY: usize = 0xDB;
    pub const EGG_COLOR: usize = 0xDC;
    pub const BODY_TYPE: usize = 0xDD;
    pub const BODY_TYPE_ANIMAL: usize = 0xDE;
    pub const DOCTOR_MEDAL: usize = 0x108;
    pub const ANIMAL_BEHAVIOR: usize = 0x118;
    /// Arm, ear, forehead, horn, leg, tail, wing, face.
    pub const ANIMAL_PARTS: usize = 0x11C;
    pub const EMOTION: usize = 0x124;
    pub const JOY: usize = 0x12C;
    pub const SLEEPINESS: usize = 0x134;
    pub const HUNGER: usize = 0x138;
    pub const PERSONALITY: usize = 0x14A;
    pub const FAVORITE_FRUIT: usize = 0x157;
    pub const CLASSROOM_FLAGS: usize = 0x160;
    pub const TOYS: usize = 0x164;
    pub const CHARACTER_BONDS: usize = 0x16C;
    pub const CHARACTER_BOND_SIZE: usize = 6;
    pub const DNA: usize = 0x438;
}

#[derive(Clone, PartialEq, Eq)]
pub struct Chao {
    pub raw: Box<[u8; SLOT_SIZE]>,
}

impl std::fmt::Debug for Chao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chao").field("type", &self.chao_type()).field("name", &self.name_bytes()).finish()
    }
}

impl Chao {
    pub fn empty() -> Chao {
        Chao { raw: Box::new([0; SLOT_SIZE]) }
    }

    pub fn from_bytes(b: &[u8]) -> Result<Chao> {
        let raw: [u8; SLOT_SIZE] = b.get(..SLOT_SIZE).and_then(|s| s.try_into().ok()).ok_or_else(|| Error::Format(sa_formats::Error::Eof(b.len())))?;
        Ok(Chao { raw: Box::new(raw) })
    }

    fn u16(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.raw[o], self.raw[o + 1]])
    }
    fn i16(&self, o: usize) -> i16 {
        self.u16(o) as i16
    }
    fn set_u16(&mut self, o: usize, v: u16) {
        self.raw[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn u32(&self, o: usize) -> u32 {
        u32::from_le_bytes(self.raw[o..o + 4].try_into().unwrap())
    }
    fn f32(&self, o: usize) -> f32 {
        f32::from_le_bytes(self.raw[o..o + 4].try_into().unwrap())
    }
    fn set_f32(&mut self, o: usize, v: f32) {
        self.raw[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn is_empty(&self) -> bool {
        self.raw[off::TYPE] == ChaoType::Empty as u8
    }

    /// `None` for an unknown type byte (e.g. a CWE-added type).
    pub fn chao_type(&self) -> Option<ChaoType> {
        ChaoType::from_u8(self.raw[off::TYPE])
    }
    pub fn set_chao_type(&mut self, t: ChaoType) {
        self.raw[off::TYPE] = t as u8;
    }

    /// Name in the game's own character encoding (not ASCII), NUL-padded.
    pub fn name_bytes(&self) -> &[u8] {
        &self.raw[off::NAME..off::NAME + off::NAME_LEN]
    }
    pub fn set_name_bytes(&mut self, name: &[u8]) {
        let dst = &mut self.raw[off::NAME..off::NAME + off::NAME_LEN];
        dst.fill(0);
        let n = name.len().min(off::NAME_LEN);
        dst[..n].copy_from_slice(&name[..n]);
    }

    pub fn garden(&self) -> u8 {
        self.raw[off::GARDEN]
    }
    pub fn happiness(&self) -> i16 {
        self.i16(off::HAPPINESS)
    }
    pub fn set_happiness(&mut self, v: i16) {
        self.set_u16(off::HAPPINESS, v as u16);
    }
    pub fn lifespan(&self) -> i16 {
        self.i16(off::LIFESPAN)
    }
    pub fn reincarnations(&self) -> i16 {
        self.i16(off::REINCARNATIONS)
    }
    pub fn seed(&self) -> u32 {
        self.u32(off::SEED)
    }

    /// Grade (E=0 .. S=5, X=6 with CWE).
    pub fn stat_grade(&self, s: ChaoStat) -> u8 {
        self.raw[off::STAT_GRADES + s as usize]
    }
    pub fn stat_level(&self, s: ChaoStat) -> u8 {
        self.raw[off::STAT_LEVELS + s as usize]
    }
    pub fn stat_points(&self, s: ChaoStat) -> u16 {
        self.u16(off::STAT_POINTS + 2 * s as usize)
    }
    /// Progress toward the next level (0..=99 in the base game).
    pub fn stat_fraction(&self, s: ChaoStat) -> u8 {
        self.raw[off::STAT_FRACTIONS + s as usize]
    }
    pub fn set_stat(&mut self, s: ChaoStat, grade: u8, level: u8, points: u16) {
        self.raw[off::STAT_GRADES + s as usize] = grade;
        self.raw[off::STAT_LEVELS + s as usize] = level;
        self.set_u16(off::STAT_POINTS + 2 * s as usize, points);
    }

    /// Negative = Power, positive = Run.
    pub fn power_run(&self) -> f32 {
        self.f32(off::POWER_RUN)
    }
    /// Negative = Fly, positive = Swim.
    pub fn fly_swim(&self) -> f32 {
        self.f32(off::FLY_SWIM)
    }
    /// Negative = Dark, positive = Hero.
    pub fn alignment(&self) -> f32 {
        self.f32(off::ALIGNMENT)
    }
    pub fn set_alignment(&mut self, v: f32) {
        self.set_f32(off::ALIGNMENT, v);
    }
    pub fn evolution_progress(&self) -> f32 {
        self.f32(off::EVOLUTION_PROGRESS)
    }
    pub fn set_evolution_progress(&mut self, v: f32) {
        self.set_f32(off::EVOLUTION_PROGRESS, v);
    }

    pub fn color(&self) -> u8 {
        self.raw[off::COLOR]
    }
    pub fn texture(&self) -> u8 {
        self.raw[off::TEXTURE]
    }
    pub fn shiny(&self) -> bool {
        self.raw[off::SHINY] != 0
    }
    pub fn monotone(&self) -> bool {
        self.raw[off::MONOTONE] != 0
    }
    pub fn headgear(&self) -> u8 {
        self.raw[off::HEADGEAR]
    }
    pub fn medal(&self) -> u8 {
        self.raw[off::MEDAL]
    }

    /// The 8 animal-part slots: arm, ear, forehead, horn, leg, tail, wing, face.
    pub fn animal_parts(&self) -> [Option<Animal>; 8] {
        std::array::from_fn(|i| Animal::from_i8(self.raw[off::ANIMAL_PARTS + i] as i8))
    }
    pub fn set_animal_part(&mut self, slot: usize, a: Animal) {
        if slot < 8 {
            self.raw[off::ANIMAL_PARTS + slot] = a as i8 as u8;
        }
    }

    pub fn joy(&self) -> i8 {
        self.raw[off::JOY] as i8
    }
    pub fn hunger(&self) -> i16 {
        self.i16(off::HUNGER)
    }
    pub fn favorite_fruit(&self) -> u8 {
        self.raw[off::FAVORITE_FRUIT]
    }
    pub fn classroom_flags(&self) -> u32 {
        self.u32(off::CLASSROOM_FLAGS)
    }
    pub fn toys(&self) -> u16 {
        self.u16(off::TOYS)
    }

    /// Raw 6-byte bond record for each of the six playable characters.
    pub fn character_bond(&self, i: usize) -> Option<&[u8]> {
        (i < 6).then(|| &self.raw[off::CHARACTER_BONDS + i * off::CHARACTER_BOND_SIZE..][..off::CHARACTER_BOND_SIZE])
    }
}

/// An SA2 Chao save file. Keeps the whole file so writing preserves every
/// byte outside the slots too.
#[derive(Debug, Clone)]
pub struct Sa2ChaoFile {
    bytes: Vec<u8>,
    slot_offset: usize,
    pub slots: Vec<Chao>,
}

impl Sa2ChaoFile {
    pub fn read(bytes: &[u8]) -> Result<Sa2ChaoFile> {
        Self::read_at(bytes, SA2_SLOT_OFFSET)
    }

    /// Reads with an explicit slot-array offset. Reads as many complete
    /// slots as fit, up to 37.
    pub fn read_at(bytes: &[u8], slot_offset: usize) -> Result<Sa2ChaoFile> {
        let available = bytes.len().saturating_sub(slot_offset) / SLOT_SIZE;
        if available == 0 {
            return Err(Error::Format(sa_formats::Error::Eof(bytes.len())));
        }
        let slots = (0..available.min(SA2_SLOT_COUNT)).map(|i| Chao::from_bytes(&bytes[slot_offset + i * SLOT_SIZE..])).collect::<Result<_>>()?;
        Ok(Sa2ChaoFile { bytes: bytes.to_vec(), slot_offset, slots })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = self.bytes.clone();
        for (i, c) in self.slots.iter().enumerate() {
            let o = self.slot_offset + i * SLOT_SIZE;
            out[o..o + SLOT_SIZE].copy_from_slice(&c.raw[..]);
        }
        out
    }

    pub fn occupied(&self) -> impl Iterator<Item = (usize, &Chao)> {
        self.slots.iter().enumerate().filter(|(_, c)| !c.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_with(chao: &Chao, slot: usize) -> Vec<u8> {
        let mut f: Vec<u8> = (0..SA2_SLOT_OFFSET + SA2_SLOT_COUNT * SLOT_SIZE + 16).map(|i| (i * 7) as u8).collect();
        for s in 0..SA2_SLOT_COUNT {
            f[SA2_SLOT_OFFSET + s * SLOT_SIZE + off::TYPE] = 0;
        }
        f[SA2_SLOT_OFFSET + slot * SLOT_SIZE..][..SLOT_SIZE].copy_from_slice(&chao.raw[..]);
        f
    }

    #[test]
    fn typed_fields_round_trip() {
        let mut c = Chao::empty();
        c.set_chao_type(ChaoType::HeroFly);
        c.set_name_bytes(&[0x21, 0x22, 0x23]);
        c.set_stat(ChaoStat::Fly, 5, 99, 3266);
        c.set_alignment(1.0);
        c.set_happiness(80);
        c.set_animal_part(6, Animal::Peacock);
        assert_eq!(c.chao_type(), Some(ChaoType::HeroFly));
        assert_eq!(c.stat_points(ChaoStat::Fly), 3266);
        assert_eq!(c.stat_grade(ChaoStat::Fly), 5);
        assert_eq!(c.alignment(), 1.0);
        assert_eq!(c.happiness(), 80);
        assert_eq!(c.animal_parts()[6], Some(Animal::Peacock));
        assert_eq!(&c.name_bytes()[..4], &[0x21, 0x22, 0x23, 0]);
    }

    #[test]
    fn file_round_trip_is_byte_identical() {
        let mut c = Chao::empty();
        c.set_chao_type(ChaoType::Child);
        c.raw[0x7F0] = 0xAB; // unknown byte must survive
        let bytes = file_with(&c, 3);
        let f = Sa2ChaoFile::read(&bytes).unwrap();
        assert_eq!(f.slots.len(), SA2_SLOT_COUNT);
        assert_eq!(f.occupied().map(|(i, _)| i).collect::<Vec<_>>(), vec![3]);
        assert_eq!(f.write(), bytes);
    }

    #[test]
    fn edits_land_in_the_right_slot() {
        let bytes = file_with(&Chao::empty(), 0);
        let mut f = Sa2ChaoFile::read(&bytes).unwrap();
        f.slots[2].set_chao_type(ChaoType::Egg);
        let out = f.write();
        assert_eq!(out[SA2_SLOT_OFFSET + 2 * SLOT_SIZE + off::TYPE], ChaoType::Egg as u8);
        assert_eq!(out.len(), bytes.len());
    }

    #[test]
    fn too_small_file_errors() {
        assert!(Sa2ChaoFile::read(&[0u8; 100]).is_err());
    }

    #[test]
    fn offsets_match_struct_layout() {
        // Emotion struct is 0x3C bytes: lesson flags follow at 0x160.
        assert_eq!(off::EMOTION + 0x3C, off::CLASSROOM_FLAGS);
        assert_eq!(off::CHARACTER_BONDS + 6 * off::CHARACTER_BOND_SIZE, 0x190);
        assert_eq!(off::DNA, 0x190 + 680);
    }
}
