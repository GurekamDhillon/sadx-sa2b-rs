//! Stat level bars, levels and points.
//!
//! Verified (Chao Island, stats page and wiki):
//! - each stat has a level bar; reaching 100% levels the stat up, empties
//!   the bar and adds points by grade;
//! - points per level = `grade * 3 + 13 ± 2` (uniform), grade E=0 .. S=5;
//! - level cap 99; points cap 3266 (Swim/Fly/Run/Power/Stamina) and 4000
//!   (Luck/Intelligence) on modern versions.
//!
//! Unverified: what happens when a bar would go below 0 (clamped here) and
//! whether a bar at level 99 keeps filling (held at 100% here).

use sa_data::chao_save::Chao;
use sa_data::enums::ChaoStat;

use crate::rng::Rng;

pub const LEVEL_CAP: u8 = 99;

pub fn points_cap(stat: ChaoStat) -> u16 {
    match stat {
        ChaoStat::Luck | ChaoStat::Intelligence => 4000,
        _ => 3266,
    }
}

/// Points gained on one level-up at `grade` (E=0 .. S=5, X=6 with CWE).
pub fn level_gain(grade: u8, rng: &mut Rng) -> u16 {
    (grade as i32 * 3 + 13 + rng.range(-2, 2)).max(0) as u16
}

/// Adds `percent` of a level bar to `stat` (negative lowers it). Returns
/// the number of levels gained.
pub fn add_progress(chao: &mut Chao, stat: ChaoStat, percent: i32, rng: &mut Rng) -> u32 {
    let mut bar = chao.stat_fraction(stat) as i32 + percent;
    let mut level = chao.stat_level(stat);
    let grade = chao.stat_grade(stat);
    let mut points = chao.stat_points(stat);
    let mut gained = 0;
    while bar >= 100 && level < LEVEL_CAP {
        bar -= 100;
        level += 1;
        points = (points + level_gain(grade, rng)).min(points_cap(stat));
        gained += 1;
    }
    if level >= LEVEL_CAP {
        bar = bar.min(100);
    }
    chao.set_stat(stat, grade, level, points);
    chao.set_stat_fraction(stat, bar.clamp(0, 100) as u8);
    gained
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chao() -> Chao {
        let mut c = Chao::empty();
        c.set_stat(ChaoStat::Swim, 5, 1, 0); // S grade
        c
    }

    #[test]
    fn gain_matches_formula_bounds() {
        let mut rng = Rng::new(7);
        for grade in 0..=5u8 {
            for _ in 0..200 {
                let g = level_gain(grade, &mut rng) as i32;
                let mid = grade as i32 * 3 + 13;
                assert!((mid - 2..=mid + 2).contains(&g), "grade {grade}: {g}");
            }
        }
    }

    #[test]
    fn bar_levels_up_and_carries_over() {
        let mut c = chao();
        let mut rng = Rng::new(1);
        assert_eq!(add_progress(&mut c, ChaoStat::Swim, 60, &mut rng), 0);
        assert_eq!(c.stat_fraction(ChaoStat::Swim), 60);
        assert_eq!(add_progress(&mut c, ChaoStat::Swim, 60, &mut rng), 1);
        assert_eq!(c.stat_fraction(ChaoStat::Swim), 20);
        assert_eq!(c.stat_level(ChaoStat::Swim), 2);
        assert!((26..=30).contains(&c.stat_points(ChaoStat::Swim)));
    }

    #[test]
    fn negative_progress_clamps_at_zero() {
        let mut c = chao();
        let mut rng = Rng::new(1);
        add_progress(&mut c, ChaoStat::Swim, -50, &mut rng);
        assert_eq!(c.stat_fraction(ChaoStat::Swim), 0);
        assert_eq!(c.stat_level(ChaoStat::Swim), 1);
    }

    #[test]
    fn caps_at_level_99_and_points_cap() {
        let mut c = chao();
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            add_progress(&mut c, ChaoStat::Swim, 100, &mut rng);
        }
        assert_eq!(c.stat_level(ChaoStat::Swim), 99);
        assert!(c.stat_points(ChaoStat::Swim) <= 3266);
        assert!(c.stat_fraction(ChaoStat::Swim) <= 100);
    }
}
