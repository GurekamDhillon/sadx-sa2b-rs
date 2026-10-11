//! Feeding fruit and giving animals / Chaos Drives.
//!
//! Animal stat changes are Chao Island's SA2 table
//! (<https://chao-island.com/info-center/training-sa2/animals.html>), given
//! there in tenths of a level bar; on modern versions they are percentages
//! (Otter +4.4 Swim = +44%, matching the modern-games table on the SA1
//! animals page). Every animal also adds 20% to Luck and Intelligence
//! (modern games). Chaos Drives add 24% to their stat (SA2 Chaos Drives
//! page) and raise Luck twice as fast as animals (stats page). Garden fruit
//! gives +40% Stamina (feeding page).

use sa_data::chao_save::Chao;
use sa_data::enums::{Animal, ChaoStat};

use crate::rng::Rng;
use crate::stats::add_progress;
use crate::tuning::Tuning;

/// Evolution group of an animal (the colour of its icon).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Yellow: pushes toward Swim.
    Swim,
    /// Purple: pushes toward Fly.
    Fly,
    /// Green: pushes toward Run.
    Run,
    /// Red: pushes toward Power.
    Power,
    /// Blue / gold / black (random, mythical, ghost): pushes toward a random type.
    Random,
}

/// (group, [swim, fly, run, power] in % of a level bar).
pub fn animal_effect(a: Animal) -> Option<(Group, [i32; 4])> {
    use Animal::*;
    use Group::*;
    Some(match a {
        Otter => (Swim, [44, -4, 8, -16]),
        Penguin => (Swim, [36, 12, 8, -24]),
        Seal => (Swim, [40, -16, 0, 8]),
        Condor => (Fly, [20, 60, -24, 16]),
        Parrot => (Fly, [0, 48, -20, 4]),
        Peacock => (Fly, [12, 48, -16, -12]),
        Cheetah => (Run, [-8, -8, 40, 8]),
        Warthog => (Run, [-4, -12, 32, 16]), // "Boar" on Chao Island
        Rabbit => (Run, [0, 8, 40, -16]),
        Bear => (Power, [8, -16, 4, 36]),
        Gorilla => (Power, [-4, -8, 4, 40]),
        Tiger => (Power, [-8, -16, 20, 36]),
        Raccoon => (Random, [20, 12, 4, 8]),
        Sheep => (Random, [8, 12, 20, 20]),
        Skunk => (Random, [8, 12, 12, 8]),
        Dragon => (Random, [20, 4, 8, 32]),
        Phoenix => (Random, [12, 32, 4, 16]),
        Unicorn => (Random, [16, 12, 36, 0]),
        Bat => (Random, [8, 40, 8, 8]),
        HalfFish => (Random, [32, 0, 8, 24]),
        SkeletonDog => (Random, [8, 8, 32, 16]),
        _ => return Option::None,
    })
}

const MAIN_STATS: [ChaoStat; 4] = [ChaoStat::Swim, ChaoStat::Fly, ChaoStat::Run, ChaoStat::Power];

fn push_axis(chao: &mut Chao, group: Group, t: &Tuning, rng: &mut Rng) {
    let group = if group == Group::Random { [Group::Swim, Group::Fly, Group::Run, Group::Power][rng.range(0, 3) as usize] } else { group };
    let s = t.axis_step;
    match group {
        // FlySwim: negative = Fly, positive = Swim. PowerRun: negative = Power, positive = Run.
        Group::Swim => chao.set_fly_swim((chao.fly_swim() + s).min(1.0)),
        Group::Fly => chao.set_fly_swim((chao.fly_swim() - s).max(-1.0)),
        Group::Run => chao.set_power_run((chao.power_run() + s).min(1.0)),
        Group::Power => chao.set_power_run((chao.power_run() - s).max(-1.0)),
        Group::Random => unreachable!(),
    }
}

/// Gives an animal: stat bars, Luck/Intelligence, evolution axis, and marks
/// the animal as absorbed this life (`SA2AnimalBehavior` bit).
pub fn apply_animal(chao: &mut Chao, animal: Animal, t: &Tuning, rng: &mut Rng) -> bool {
    let Some((group, deltas)) = animal_effect(animal) else { return false };
    for (stat, d) in MAIN_STATS.iter().zip(deltas) {
        add_progress(chao, *stat, d, rng);
    }
    add_progress(chao, ChaoStat::Luck, 20, rng);
    add_progress(chao, ChaoStat::Intelligence, 20, rng);
    push_axis(chao, group, t, rng);
    chao.set_animal_behavior(chao.animal_behavior() | 1 << animal as u32);
    true
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosDrive {
    Yellow = 0,
    Green = 1,
    Red = 2,
    Purple = 3,
}

pub fn apply_drive(chao: &mut Chao, drive: ChaosDrive, t: &Tuning, rng: &mut Rng) {
    let (stat, group) = match drive {
        ChaosDrive::Yellow => (ChaoStat::Swim, Group::Swim),
        ChaosDrive::Green => (ChaoStat::Run, Group::Run),
        ChaosDrive::Red => (ChaoStat::Power, Group::Power),
        ChaosDrive::Purple => (ChaoStat::Fly, Group::Fly),
    };
    add_progress(chao, stat, 24, rng);
    add_progress(chao, ChaoStat::Luck, 40, rng);
    add_progress(chao, ChaoStat::Intelligence, t.drive_intelligence_percent, rng);
    push_axis(chao, group, t, rng);
}

/// SA2 fruit ids (`SA2BFruit`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fruit {
    ChaoGarden = 0x0,
    HeroGarden = 0x1,
    DarkGarden = 0x2,
    Strong = 0x3,
    Tasty = 0x4,
    Hero = 0x5,
    Dark = 0x6,
    Round = 0x7,
    Triangle = 0x8,
    Square = 0x9,
    Heart = 0xA,
    Chao = 0xB,
    Smart = 0xC,
    Orange = 0xD,
    Blue = 0xE,
    Pink = 0xF,
    Green = 0x10,
    Purple = 0x11,
    Yellow = 0x12,
    Red = 0x13,
    Mushroom = 0x14,
    MushroomAlt = 0x15,
    MintCandy = 0x16,
    Grapes = 0x17,
}

/// Feeds a whole fruit (all four bites). Returns true for a Heart fruit
/// (the Chao becomes ready to mate; mating is not simulated here).
pub fn apply_fruit(chao: &mut Chao, fruit: Fruit, t: &Tuning, rng: &mut Rng) -> bool {
    // Verified: garden fruit +40% Stamina. Other fruit: Stamina +40% assumed
    // ("any fruit raises Stamina", amounts for shop fruit not published).
    let stamina = match fruit {
        Fruit::Mushroom | Fruit::MushroomAlt => t.mushroom_stamina_percent,
        _ => 40,
    };
    add_progress(chao, ChaoStat::Stamina, stamina, rng);
    match fruit {
        Fruit::Hero => chao.set_alignment((chao.alignment() + t.fruit_alignment_step).min(1.0)),
        Fruit::Dark => chao.set_alignment((chao.alignment() - t.fruit_alignment_step).max(-1.0)),
        Fruit::Chao => {
            for s in MAIN_STATS {
                add_progress(chao, s, t.chao_fruit_percent, rng);
            }
        }
        Fruit::Smart => {
            add_progress(chao, ChaoStat::Intelligence, t.smart_fruit_percent, rng);
        }
        Fruit::Mushroom | Fruit::MushroomAlt => {
            add_progress(chao, ChaoStat::Luck, t.mushroom_luck_percent, rng);
        }
        _ => {}
    }
    fruit == Fruit::Heart
}

/// A Hero (+1) or Dark (−1) character petting or feeding the Chao.
pub fn care(chao: &mut Chao, side: i8, t: &Tuning) {
    let a = chao.alignment() + t.care_alignment_step * side.signum() as f32;
    chao.set_alignment(a.clamp(-1.0, 1.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_real_animal_has_an_effect() {
        for v in 0..=0x14 {
            assert!(animal_effect(Animal::from_i8(v).unwrap()).is_some(), "animal {v}");
        }
        assert!(animal_effect(Animal::YellowChaosDrive).is_none());
    }

    #[test]
    fn otter_raises_swim_lowers_power_and_pushes_swim() {
        let mut c = Chao::empty();
        let mut rng = Rng::new(5);
        apply_animal(&mut c, Animal::Otter, &Tuning::default(), &mut rng);
        assert_eq!(c.stat_fraction(ChaoStat::Swim), 44);
        assert_eq!(c.stat_fraction(ChaoStat::Power), 0);
        assert_eq!(c.stat_fraction(ChaoStat::Luck), 20);
        assert!(c.fly_swim() > 0.0);
        assert_ne!(c.animal_behavior() & 1 << Animal::Otter as u32, 0);
    }

    #[test]
    fn five_drives_level_a_stat() {
        // Chao Island: four drives nearly fill a bar; five level it up.
        let mut c = Chao::empty();
        let mut rng = Rng::new(5);
        let t = Tuning::default();
        for _ in 0..4 {
            apply_drive(&mut c, ChaosDrive::Red, &t, &mut rng);
        }
        assert_eq!((c.stat_level(ChaoStat::Power), c.stat_fraction(ChaoStat::Power)), (0, 96));
        apply_drive(&mut c, ChaosDrive::Red, &t, &mut rng);
        assert_eq!(c.stat_level(ChaoStat::Power), 1);
        assert!(c.power_run() < 0.0);
    }

    #[test]
    fn garden_fruit_and_hero_fruit() {
        let mut c = Chao::empty();
        let mut rng = Rng::new(5);
        let t = Tuning::default();
        apply_fruit(&mut c, Fruit::ChaoGarden, &t, &mut rng);
        assert_eq!(c.stat_fraction(ChaoStat::Stamina), 40);
        apply_fruit(&mut c, Fruit::Hero, &t, &mut rng);
        assert!(c.alignment() > 0.0);
        assert!(apply_fruit(&mut c, Fruit::Heart, &t, &mut rng));
    }
}
