//! Numbers that are NOT verified against the game or a primary source.
//! Defaults are reasoned estimates; each field says what is known.

#[derive(Debug, Clone, PartialEq)]
pub struct Tuning {
    /// How far one animal or drive of a common colour moves the matching
    /// evolution axis (`PowerRun` / `FlySwim`, each -1..1). Direction is
    /// verified (yellow→Swim, purple→Fly, green→Run, red→Power); the step is not.
    pub axis_step: f32,
    /// Alignment change per care action (pet/feed) by a Hero (+) or Dark (−)
    /// character. Direction verified (Chao Island alignment page); size not.
    pub care_alignment_step: f32,
    /// Alignment change per Hero/Dark fruit ("slightly"; size not published).
    pub fruit_alignment_step: f32,
    /// Chao Fruit: bar % added to each of Swim/Fly/Run/Power (amount not published).
    pub chao_fruit_percent: i32,
    /// Mushroom: Stamina and Luck bar % ("significantly" / "also"; not published).
    pub mushroom_stamina_percent: i32,
    pub mushroom_luck_percent: i32,
    /// Smart Fruit: Intelligence bar % (not published).
    pub smart_fruit_percent: i32,
    /// Chaos Drive Intelligence bar %. Chao Island says drives raise
    /// Intelligence but gives no amount; animals give 20%.
    pub drive_intelligence_percent: i32,
    /// Evolution progress added per simulated minute while a child (0..1;
    /// cocoon at 1.0). Chao Island: cocoons after "a few hours". Default ≈ 3 h.
    pub evolution_per_minute: f32,
    /// Lifespan units lost per simulated minute. Chao Island: lifespan ≈ 5
    /// chao years ≈ 15 real hours. The save's starting `Lifespan` value is
    /// unverified; with the default start of 3000 this is 3000 / 900 min.
    pub lifespan_per_minute: f32,
    /// Starting `Lifespan` for a new life (unverified, community figure).
    pub lifespan_start: i16,
    /// |axis| at or above which the axis decides the evolution type; below
    /// it on both axes the Chao evolves Normal. Not published.
    pub type_threshold: f32,
    /// |Alignment| at or above which the Chao becomes Hero/Dark. Not published.
    pub alignment_threshold: f32,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            axis_step: 0.05,
            care_alignment_step: 0.01,
            fruit_alignment_step: 0.05,
            chao_fruit_percent: 10,
            mushroom_stamina_percent: 80,
            mushroom_luck_percent: 40,
            smart_fruit_percent: 40,
            drive_intelligence_percent: 20,
            evolution_per_minute: 1.0 / 180.0,
            lifespan_per_minute: 3000.0 / 900.0,
            lifespan_start: 3000,
            type_threshold: 0.5,
            alignment_threshold: 0.5,
        }
    }
}
