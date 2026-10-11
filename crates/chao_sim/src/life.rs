//! Growing up, evolving, aging, reincarnation and death.
//!
//! Verified (Chao Island life-cycle pages):
//! - adult type comes from recent animal/drive input (the `PowerRun` /
//!   `FlySwim` axes); balanced or no input gives Normal;
//! - alignment -1 Dark .. +1 Hero picks Hero/Neutral/Dark;
//! - Chaos Chao: 3rd life or later, happiness > 80, at least one of every
//!   animal in this life, and otherwise Normal-type; alignment then picks
//!   Light (Neutral) / Angel (Hero) / Devil (Dark);
//! - lifespan about 5 chao years (≈15 real hours); at the end the Chao
//!   cocoons: pink (happiness > 50) = reincarnates, grey = disappears;
//! - reincarnation keeps grades, resets levels to 1 and points to floor(10%).
//!
//! Unverified numbers (thresholds, rates) come from [`Tuning`].

use sa_data::chao_save::Chao;
use sa_data::enums::{ChaoStat, ChaoType};

use crate::tuning::Tuning;

/// Bits of `SA2AnimalBehavior` for the 21 real animals (Penguin..Phoenix).
pub const ALL_ANIMALS: u32 = (1 << 21) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Neutral,
    Hero,
    Dark,
}

pub fn alignment_of(chao: &Chao, t: &Tuning) -> Alignment {
    let a = chao.alignment();
    if a >= t.alignment_threshold {
        Alignment::Hero
    } else if a <= -t.alignment_threshold {
        Alignment::Dark
    } else {
        Alignment::Neutral
    }
}

fn typed(base: u8, align: Alignment) -> ChaoType {
    // ChaoType groups are laid out Neutral, Hero, Dark for each form.
    let offset = match align {
        Alignment::Neutral => 0,
        Alignment::Hero => 1,
        Alignment::Dark => 2,
    };
    ChaoType::from_u8(base + offset).expect("valid ChaoType")
}

/// The adult type a child would become now.
pub fn evolution_type(chao: &Chao, t: &Tuning) -> ChaoType {
    let align = alignment_of(chao, t);
    let pr = chao.power_run();
    let fs = chao.fly_swim();
    let normal = pr.abs() < t.type_threshold && fs.abs() < t.type_threshold;
    if normal
        && chao.reincarnations() >= 2
        && chao.happiness() > 80
        && chao.animal_behavior() & ALL_ANIMALS == ALL_ANIMALS
    {
        return typed(ChaoType::NeutralChaos as u8, align);
    }
    if normal {
        return typed(ChaoType::NeutralNormal as u8, align);
    }
    let base = if fs.abs() >= pr.abs() {
        if fs > 0.0 { ChaoType::NeutralSwim } else { ChaoType::NeutralFly }
    } else if pr > 0.0 {
        ChaoType::NeutralRun
    } else {
        ChaoType::NeutralPower
    };
    typed(base as u8, align)
}

/// Turns a child into its adult type. Returns the new type, or `None` if
/// the Chao is not a child.
pub fn evolve(chao: &mut Chao, t: &Tuning) -> Option<ChaoType> {
    if chao.chao_type() != Some(ChaoType::Child) {
        return None;
    }
    let ty = evolution_type(chao, t);
    chao.set_chao_type(ty);
    chao.set_evolution_progress(0.0);
    Some(ty)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CocoonOutcome {
    /// Pink cocoon: reborn as a child with carried-over stats.
    Reincarnate,
    /// Grey cocoon: the Chao disappears.
    Disappear,
}

/// What happens when this Chao's lifespan runs out.
pub fn cocoon_outcome(chao: &Chao) -> CocoonOutcome {
    if chao.happiness() > 50 { CocoonOutcome::Reincarnate } else { CocoonOutcome::Disappear }
}

/// Applies a lifespan-end cocoon to the slot.
pub fn end_of_life(chao: &mut Chao, t: &Tuning) -> CocoonOutcome {
    let outcome = cocoon_outcome(chao);
    match outcome {
        CocoonOutcome::Reincarnate => {
            for s in ChaoStat::ALL {
                let points = chao.stat_points(s) / 10;
                chao.set_stat(s, chao.stat_grade(s), 1, points);
                chao.set_stat_fraction(s, 0);
            }
            chao.set_reincarnations(chao.reincarnations() + 1);
            chao.set_last_life_alignment(chao.alignment());
            chao.set_chao_type(ChaoType::Child);
            chao.set_evolution_progress(0.0);
            chao.set_power_run(0.0);
            chao.set_fly_swim(0.0);
            chao.set_animal_behavior(0);
            chao.set_lifespan(t.lifespan_start);
        }
        CocoonOutcome::Disappear => *chao = Chao::empty(),
    }
    outcome
}

/// Events produced by [`age`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeEvent {
    Evolved(ChaoType),
    Cocoon(CocoonOutcome),
}

/// Advances time by `minutes`: children progress toward evolution; everyone
/// loses lifespan (Chaos Chao excepted, unverified). Applies the resulting
/// evolution or end of life and reports it.
pub fn age(chao: &mut Chao, minutes: f32, t: &Tuning) -> Option<LifeEvent> {
    let ty = chao.chao_type()?;
    if matches!(ty, ChaoType::Empty | ChaoType::Egg) {
        return None;
    }
    if ty == ChaoType::Child {
        let p = chao.evolution_progress() + t.evolution_per_minute * minutes;
        chao.set_evolution_progress(p.min(1.0));
        if p >= 1.0 {
            return evolve(chao, t).map(LifeEvent::Evolved);
        }
    }
    let chaos = matches!(ty, ChaoType::NeutralChaos | ChaoType::HeroChaos | ChaoType::DarkChaos);
    if !chaos {
        let left = chao.lifespan() as f32 - t.lifespan_per_minute * minutes;
        chao.set_lifespan(left.max(0.0) as i16);
        if left <= 0.0 {
            return Some(LifeEvent::Cocoon(end_of_life(chao, t)));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child() -> Chao {
        let mut c = Chao::empty();
        c.set_chao_type(ChaoType::Child);
        c.set_lifespan(3000);
        c
    }

    #[test]
    fn balanced_child_evolves_neutral_normal() {
        let mut c = child();
        assert_eq!(evolve(&mut c, &Tuning::default()), Some(ChaoType::NeutralNormal));
    }

    #[test]
    fn axes_and_alignment_pick_the_type() {
        let t = Tuning::default();
        let mut c = child();
        c.set_fly_swim(-0.8);
        c.set_alignment(1.0);
        assert_eq!(evolution_type(&c, &t), ChaoType::HeroFly);
        c.set_fly_swim(0.0);
        c.set_power_run(0.9);
        c.set_alignment(-0.9);
        assert_eq!(evolution_type(&c, &t), ChaoType::DarkRun);
        c.set_power_run(-0.9);
        c.set_alignment(0.0);
        assert_eq!(evolution_type(&c, &t), ChaoType::NeutralPower);
    }

    #[test]
    fn chaos_chao_needs_every_condition() {
        let t = Tuning::default();
        let mut c = child();
        c.set_reincarnations(2);
        c.set_happiness(90);
        c.set_animal_behavior(ALL_ANIMALS);
        c.set_alignment(1.0);
        assert_eq!(evolution_type(&c, &t), ChaoType::HeroChaos);
        c.set_happiness(80);
        assert_eq!(evolution_type(&c, &t), ChaoType::HeroNormal);
        c.set_happiness(90);
        c.set_reincarnations(1);
        assert_eq!(evolution_type(&c, &t), ChaoType::HeroNormal);
        c.set_reincarnations(2);
        c.set_animal_behavior(ALL_ANIMALS & !1);
        assert_eq!(evolution_type(&c, &t), ChaoType::HeroNormal);
    }

    #[test]
    fn child_evolves_after_about_three_hours() {
        let t = Tuning::default();
        let mut c = child();
        assert_eq!(age(&mut c, 170.0, &t), None);
        assert_eq!(age(&mut c, 20.0, &t), Some(LifeEvent::Evolved(ChaoType::NeutralNormal)));
    }

    #[test]
    fn happy_chao_reincarnates_with_carryover() {
        let t = Tuning::default();
        let mut c = child();
        c.set_chao_type(ChaoType::HeroSwim);
        c.set_happiness(60);
        c.set_stat(ChaoStat::Swim, 4, 50, 1234);
        let ev = age(&mut c, 10_000.0, &t);
        assert_eq!(ev, Some(LifeEvent::Cocoon(CocoonOutcome::Reincarnate)));
        assert_eq!(c.chao_type(), Some(ChaoType::Child));
        assert_eq!((c.stat_grade(ChaoStat::Swim), c.stat_level(ChaoStat::Swim), c.stat_points(ChaoStat::Swim)), (4, 1, 123));
        assert_eq!(c.reincarnations(), 1);
    }

    #[test]
    fn unhappy_chao_disappears() {
        let t = Tuning::default();
        let mut c = child();
        c.set_chao_type(ChaoType::DarkPower);
        c.set_happiness(10);
        assert_eq!(age(&mut c, 10_000.0, &t), Some(LifeEvent::Cocoon(CocoonOutcome::Disappear)));
        assert!(c.is_empty());
    }
}
