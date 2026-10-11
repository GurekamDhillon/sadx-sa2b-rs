//! Enumerations from the X-Hax SA2 mod loader headers (`SA2Enums.h`).
//!
//! Values are the game's own; `#[repr(C)]`/`#[repr(u8)]` so they can cross
//! the C ABI unchanged.

/// SA2 `Characters` enum. Also the index into the physics table.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Character {
    Sonic = 0,
    Shadow = 1,
    Tails = 2,
    Eggman = 3,
    Knuckles = 4,
    Rouge = 5,
    MechTails = 6,
    MechEggman = 7,
    Amy = 8,
    SuperSonic = 9,
    SuperShadow = 10,
    /// Unused slot in the retail game.
    B = 11,
    MetalSonic = 12,
    ChaoWalker = 13,
    DarkChaoWalker = 14,
    Tikal = 15,
    Chaos = 16,
}

impl Character {
    pub const ALL: [Character; 17] = [
        Character::Sonic,
        Character::Shadow,
        Character::Tails,
        Character::Eggman,
        Character::Knuckles,
        Character::Rouge,
        Character::MechTails,
        Character::MechEggman,
        Character::Amy,
        Character::SuperSonic,
        Character::SuperShadow,
        Character::B,
        Character::MetalSonic,
        Character::ChaoWalker,
        Character::DarkChaoWalker,
        Character::Tikal,
        Character::Chaos,
    ];

    pub fn from_index(i: u32) -> Option<Character> {
        Self::ALL.get(i as usize).copied()
    }
}

/// SA2 `Upgrades` bit flags (bit n = n-th entry of `UpgradeBits`).
pub mod upgrades {
    pub const SONIC_LIGHT_SHOES: u32 = 1 << 0;
    pub const SONIC_ANCIENT_LIGHT: u32 = 1 << 1;
    pub const SONIC_MAGIC_GLOVES: u32 = 1 << 2;
    pub const SONIC_FLAME_RING: u32 = 1 << 3;
    pub const SONIC_BOUNCE_BRACELET: u32 = 1 << 4;
    pub const SONIC_MYSTIC_MELODY: u32 = 1 << 5;
    pub const TAILS_BOOSTER: u32 = 1 << 6;
    pub const TAILS_BAZOOKA: u32 = 1 << 7;
    pub const TAILS_LASER_BLASTER: u32 = 1 << 8;
    pub const TAILS_MYSTIC_MELODY: u32 = 1 << 9;
    pub const KNUCKLES_SHOVEL_CLAW: u32 = 1 << 10;
    pub const KNUCKLES_SUNGLASSES: u32 = 1 << 11;
    pub const KNUCKLES_HAMMER_GLOVES: u32 = 1 << 12;
    pub const KNUCKLES_AIR_NECKLACE: u32 = 1 << 13;
    pub const KNUCKLES_MYSTIC_MELODY: u32 = 1 << 14;
    pub const SUPER_SONIC: u32 = 1 << 15;
    pub const SHADOW_AIR_SHOES: u32 = 1 << 16;
    pub const SHADOW_ANCIENT_LIGHT: u32 = 1 << 17;
    pub const SHADOW_FLAME_RING: u32 = 1 << 18;
    pub const SHADOW_MYSTIC_MELODY: u32 = 1 << 19;
    pub const EGGMAN_JET_ENGINE: u32 = 1 << 20;
    pub const EGGMAN_LARGE_CANNON: u32 = 1 << 21;
    pub const EGGMAN_LASER_BLASTER: u32 = 1 << 22;
    pub const EGGMAN_PROTECTIVE_ARMOR: u32 = 1 << 23;
    pub const EGGMAN_MYSTIC_MELODY: u32 = 1 << 24;
    pub const ROUGE_PICK_NAILS: u32 = 1 << 25;
    pub const ROUGE_TREASURE_SCOPE: u32 = 1 << 26;
    pub const ROUGE_IRON_BOOTS: u32 = 1 << 27;
    pub const ROUGE_MYSTIC_MELODY: u32 = 1 << 28;

    /// Upgrade names in bit order.
    pub const NAMES: [&str; 29] = [
        "SonicLightShoes", "SonicAncientLight", "SonicMagicGloves", "SonicFlameRing", "SonicBounceBracelet", "SonicMysticMelody",
        "TailsBooster", "TailsBazooka", "TailsLaserBlaster", "TailsMysticMelody",
        "KnucklesShovelClaw", "KnucklesSunglasses", "KnucklesHammerGloves", "KnucklesAirNecklace", "KnucklesMysticMelody",
        "SuperSonic",
        "ShadowAirShoes", "ShadowAncientLight", "ShadowFlameRing", "ShadowMysticMelody",
        "EggmanJetEngine", "EggmanLargeCannon", "EggmanLaserBlaster", "EggmanProtectiveArmor", "EggmanMysticMelody",
        "RougePickNails", "RougeTreasureScope", "RougeIronBoots", "RougeMysticMelody",
    ];
}

/// SA2 `ChaoType`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaoType {
    Empty = 0x0,
    Egg = 0x1,
    Child = 0x2,
    Good = 0x3,
    Bad = 0x4,
    NeutralNormal = 0x5,
    HeroNormal = 0x6,
    DarkNormal = 0x7,
    NeutralSwim = 0x8,
    HeroSwim = 0x9,
    DarkSwim = 0xA,
    NeutralFly = 0xB,
    HeroFly = 0xC,
    DarkFly = 0xD,
    NeutralRun = 0xE,
    HeroRun = 0xF,
    DarkRun = 0x10,
    NeutralPower = 0x11,
    HeroPower = 0x12,
    DarkPower = 0x13,
    NeutralChaos = 0x14,
    HeroChaos = 0x15,
    DarkChaos = 0x16,
    Tails = 0x17,
    Knuckles = 0x18,
    Amy = 0x19,
}

impl ChaoType {
    pub fn from_u8(v: u8) -> Option<ChaoType> {
        if v <= 0x19 {
            // SAFETY: repr(u8) with contiguous discriminants 0..=0x19.
            Some(unsafe { std::mem::transmute::<u8, ChaoType>(v) })
        } else {
            None
        }
    }
}

/// SA2B small animals and Chaos Drives (`SA2BAnimal`); -1 = none.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Animal {
    None = -1,
    Penguin = 0x0,
    Seal = 0x1,
    Otter = 0x2,
    Rabbit = 0x3,
    Cheetah = 0x4,
    Warthog = 0x5,
    Bear = 0x6,
    Tiger = 0x7,
    Gorilla = 0x8,
    Peacock = 0x9,
    Parrot = 0xA,
    Condor = 0xB,
    Skunk = 0xC,
    Sheep = 0xD,
    Raccoon = 0xE,
    HalfFish = 0xF,
    SkeletonDog = 0x10,
    Bat = 0x11,
    Dragon = 0x12,
    Unicorn = 0x13,
    Phoenix = 0x14,
    YellowChaosDrive = 0x15,
    GreenChaosDrive = 0x16,
    RedChaosDrive = 0x17,
    PurpleChaosDrive = 0x18,
}

impl Animal {
    pub fn from_i8(v: i8) -> Option<Animal> {
        if (-1..=0x18).contains(&v) {
            // SAFETY: repr(i8) with contiguous discriminants -1..=0x18.
            Some(unsafe { std::mem::transmute::<i8, Animal>(v) })
        } else {
            None
        }
    }
}

/// Chao stat order used by every per-stat array (`StatGrades[8]` etc.).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaoStat {
    Swim = 0,
    Fly = 1,
    Run = 2,
    Power = 3,
    Stamina = 4,
    Luck = 5,
    Intelligence = 6,
    Unknown = 7,
}

/// Stat grades as stored in `StatGrades`: E=0 .. S=5 (CWE adds X=6).
pub const GRADE_NAMES: [&str; 7] = ["E", "D", "C", "B", "A", "S", "X"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_indices_match_header() {
        assert_eq!(Character::Chaos as u32, 16);
        assert_eq!(Character::from_index(12), Some(Character::MetalSonic));
        assert_eq!(Character::from_index(17), None);
    }

    #[test]
    fn upgrade_names_line_up_with_bits() {
        assert_eq!(upgrades::NAMES[upgrades::SUPER_SONIC.trailing_zeros() as usize], "SuperSonic");
        assert_eq!(upgrades::NAMES[upgrades::ROUGE_MYSTIC_MELODY.trailing_zeros() as usize], "RougeMysticMelody");
    }

    #[test]
    fn chao_enum_conversions() {
        assert_eq!(ChaoType::from_u8(0x19), Some(ChaoType::Amy));
        assert_eq!(ChaoType::from_u8(0x1A), None);
        assert_eq!(Animal::from_i8(-1), Some(Animal::None));
        assert_eq!(Animal::from_i8(0x18), Some(Animal::PurpleChaosDrive));
        assert_eq!(Animal::from_i8(0x19), None);
    }
}
