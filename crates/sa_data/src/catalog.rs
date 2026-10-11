//! Which files hold each character's model, motions and textures.
//!
//! SA2 PC names, all in `resource/gd_PC` (PRS-compressed). Verified against
//! the files present in a Steam install; the meaning of `ewalk2mdl.prs` is
//! not established, so it is not mapped.

use crate::enums::Character;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Costume {
    Default = 0,
    /// The alternate costume unlocked in-game (Sonic/Shadow/Knuckles/Rouge
    /// and the Tails/Eggman walkers).
    Alternate = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterFiles {
    /// Base name, e.g. `sonic` -> `sonicmdl.prs`, `sonictex.prs`.
    pub model: &'static str,
    pub textures: &'static str,
    /// Motion file base name (motions are shared between costumes).
    pub motions: Option<&'static str>,
}

impl CharacterFiles {
    pub fn model_file(&self) -> String {
        format!("{}mdl.prs", self.model)
    }

    pub fn texture_file(&self) -> String {
        format!("{}tex.prs", self.textures)
    }

    pub fn motion_file(&self) -> Option<String> {
        self.motions.map(|m| format!("{m}mtn.prs"))
    }
}

/// Files for a character/costume, or `None` if the game has no such model.
pub fn sa2_character(c: Character, costume: Costume) -> Option<CharacterFiles> {
    use Character::*;
    let f = |model, textures, motions| Some(CharacterFiles { model, textures, motions });
    match (c, costume) {
        (Sonic, Costume::Default) => f("sonic", "sonic", Some("sonic")),
        (Sonic, Costume::Alternate) => f("sonic1", "sonic1", Some("sonic")),
        (Shadow, Costume::Default) => f("terios", "terios", Some("terios")),
        (Shadow, Costume::Alternate) => f("shadow1", "shadow1", Some("terios")),
        (Tails, Costume::Default) => f("miles", "miles", Some("miles")),
        (Eggman, Costume::Default) => f("egg", "egg", Some("egg")),
        (Knuckles, Costume::Default) => f("knuck", "knuck", Some("knuck")),
        (Knuckles, Costume::Alternate) => f("bknuck", "bknuck", Some("knuck")),
        (Rouge, Costume::Default) => f("rouge", "rouge", Some("rouge")),
        (Rouge, Costume::Alternate) => f("brouge", "brouge", Some("rouge")),
        (MechTails, Costume::Default) => f("twalk", "twalk", Some("twalk")),
        (MechTails, Costume::Alternate) => f("twalk1", "twalk1", Some("twalk")),
        (MechEggman, Costume::Default) => f("ewalk", "ewalk", Some("ewalk")),
        (MechEggman, Costume::Alternate) => f("ewalk1", "ewalk1", Some("ewalk")),
        (Amy, Costume::Default) => f("amy", "amy", Some("amy")),
        (SuperSonic, Costume::Default) => f("ssonic", "ssonic", Some("ssonic")),
        (SuperShadow, Costume::Default) => f("sshadow", "sshadow", Some("sshadow")),
        (MetalSonic, Costume::Default) => f("metalsonic", "metalsonic", Some("metalsonic")),
        (ChaoWalker, Costume::Default) => f("cwalk", "cwalk", Some("cwalk")),
        (DarkChaoWalker, Costume::Default) => f("dwalk", "dwalk", Some("dwalk")),
        (Tikal, Costume::Default) => f("tical", "tical", Some("tical")),
        (Chaos, Costume::Default) => f("chaos0", "chaos0", Some("chaos0")),
        _ => None,
    }
}

/// Motions shared by every character on foot (`plcommtn.prs`).
pub const SA2_COMMON_MOTIONS: &str = "plcommtn.prs";

/// Chao Garden landtable exports in `DLL/Win32/Data_DLL.dll`.
pub const SA2_CHAO_LANDTABLES: [&str; 11] = [
    "objLandTableLobby",
    "objLandTableEntrance",
    "objLandTableNeut",
    "objLandTableHero",
    "objLandTableDark",
    "objLandTableKinderCo",
    "objLandTableRace",
    "objLandTableRaceHero",
    "objLandTableRaceDark",
    "objLandTableChaoKarate",
    "objLandTableStadium",
];

/// Stage landtable export for an SA2 level id (`objLandTable0013` = City Escape).
pub fn sa2_stage_landtable(level: u32) -> String {
    format!("objLandTable{level:04}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names() {
        let s = sa2_character(Character::Shadow, Costume::Alternate).unwrap();
        assert_eq!(s.model_file(), "shadow1mdl.prs");
        assert_eq!(s.motion_file().unwrap(), "teriosmtn.prs");
        assert_eq!(sa2_character(Character::Amy, Costume::Alternate), None);
        assert_eq!(sa2_character(Character::B, Costume::Default), None);
        assert_eq!(sa2_stage_landtable(13), "objLandTable0013");
    }
}
