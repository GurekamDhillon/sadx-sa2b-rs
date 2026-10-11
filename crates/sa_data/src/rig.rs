//! A character ready to animate: its models, all of its motions, the
//! animation-state table from the exe, and its textures.

use std::collections::HashMap;

use sa_formats::bytes::Endian;
use sa_formats::ninja::{self, motion::Motion, object::Node, pose, Mesh};
use sa_formats::texture::Rgba;

use crate::anim::{self, AnimInfo};
use crate::catalog::{self, Costume};
use crate::enums::Character;
use crate::error::{read, Error, Result};
use crate::install::Game;

pub struct Rig {
    /// Model id → node tree (rest pose).
    pub models: HashMap<i32, Node>,
    /// Motion id → keyframes (common motions plus the character's own).
    pub motions: HashMap<u16, Motion>,
    /// Animation state → what to play (`CHARACTER_ANIMATIONS` indices).
    pub table: Vec<AnimInfo>,
    /// Textures in texture-id order.
    pub textures: Vec<(String, Rgba)>,
}

impl Game {
    pub fn character_rig(&self, c: Character, costume: Costume) -> Result<Rig> {
        let files = catalog::sa2_character(c, costume).ok_or_else(|| Error::NotFound(format!("no {costume:?} model for {c:?}")))?;
        let models = self.character_models(c, costume)?.into_iter().map(|m| (m.id, m.root)).collect();
        let mut motions: HashMap<u16, Motion> = HashMap::new();
        for name in [Some(catalog::SA2_COMMON_MOTIONS.to_string()), files.motion_file()].into_iter().flatten() {
            for m in ninja::motion::read_motion_file(&self.read_data(&name)?, Endian::Big)? {
                motions.insert(m.id, m.motion);
            }
        }
        let (addr, n) = anim::sa2_table(c).ok_or_else(|| Error::NotFound(format!("no animation table for {c:?}")))?;
        let table = anim::read_table(&read(&self.exe_path())?, addr, n)?;
        let textures = self.character_textures(c, costume)?;
        Ok(Rig { models, motions, table, textures })
    }
}

impl Rig {
    pub fn info(&self, state: usize) -> Option<&AnimInfo> {
        self.table.get(state)
    }

    /// Number of frames in the motion played by `state`.
    pub fn frame_count(&self, state: usize) -> f32 {
        self.info(state).and_then(|i| self.motions.get(&(i.anim_num as u16))).map(|m| m.frame_count as f32).unwrap_or(1.0)
    }

    /// The model drawn for `state` (`ModelNum` is a model id).
    pub fn model_id(&self, state: usize) -> Option<i32> {
        self.info(state).map(|i| i.model_num as i32)
    }

    /// The posed, flattened body for `state` at `frame`.
    ///
    /// Motions drive a skeleton with extra root nodes that the body models
    /// lack; those are borrowed from the model whose node count matches.
    pub fn mesh(&self, state: usize, frame: f32) -> Option<Mesh> {
        let info = self.info(state)?;
        let body = self.models.get(&(info.model_num as i32))?;
        let motion = self.motions.get(&(info.anim_num as u16))?;
        let have = body.count();
        let want = motion.nodes.len();
        let rooted = if have < want {
            let skeleton = self.models.values().find(|m| m.count() == want)?;
            pose::with_prefix(body, skeleton, want - have)
        } else {
            body.clone()
        };
        Some(ninja::flatten(&pose::posed(&rooted, motion, frame)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rig_answers_without_panicking() {
        let r = Rig { models: HashMap::new(), motions: HashMap::new(), table: vec![], textures: vec![] };
        assert!(r.mesh(0, 0.0).is_none());
        assert_eq!(r.frame_count(0), 1.0);
        assert_eq!(r.model_id(3), None);
    }
}
