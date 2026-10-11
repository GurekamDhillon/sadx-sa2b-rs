//! Posing a node tree with a motion: sample keyframe tracks at a frame and
//! override each node's local transform.
//!
//! Motion tracks are indexed by node in depth-first file order (the order of
//! [`Node::walk`]). Tracks the motion does not have leave the node's own
//! value alone. Rotations interpolate along the shortest arc.

use super::motion::{Key, Motion};
use super::object::{eval, Node};

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The pair of keys around `frame` and the blend between them.
fn bracket<T: Copy>(keys: &[Key<T>], frame: f32) -> Option<(T, T, f32)> {
    let first = keys.first()?;
    if keys.len() == 1 || frame <= first.frame as f32 {
        return Some((first.value, first.value, 0.0));
    }
    let last = keys.last()?;
    if frame >= last.frame as f32 {
        return Some((last.value, last.value, 0.0));
    }
    let i = keys.iter().position(|k| k.frame as f32 > frame)?;
    let (a, b) = (keys[i - 1], keys[i]);
    let span = (b.frame - a.frame).max(1) as f32;
    Some((a.value, b.value, (frame - a.frame as f32) / span))
}

pub fn sample_vec(keys: &[Key<[f32; 3]>], frame: f32) -> Option<[f32; 3]> {
    let (a, b, t) = bracket(keys, frame)?;
    Some([lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)])
}

pub fn sample_angles(keys: &[Key<[i32; 3]>], frame: f32) -> Option<[i32; 3]> {
    let (a, b, t) = bracket(keys, frame)?;
    let ch = |x: i32, y: i32| {
        // Shortest way round the 16-bit circle.
        let d = (y.wrapping_sub(x) as i16) as f32;
        (x as f32 + d * t) as i32
    };
    Some([ch(a[0], b[0]), ch(a[1], b[1]), ch(a[2], b[2])])
}

/// A copy of `root` with every animated node's transform set from `motion`
/// at `frame` (frames wrap at the motion's length).
pub fn posed(root: &Node, motion: &Motion, frame: f32) -> Node {
    let len = motion.frame_count.max(1) as f32;
    let frame = frame.rem_euclid(len);
    let mut out = root.clone();
    let mut index = 0;
    apply(&mut out, motion, frame, &mut index);
    out
}

fn apply(node: &mut Node, motion: &Motion, frame: f32, index: &mut usize) {
    if let Some(tracks) = motion.nodes.get(*index) {
        if let Some(p) = sample_vec(&tracks.position, frame) {
            node.position = p;
            node.flags &= !eval::UNIT_POS;
        }
        if let Some(r) = sample_angles(&tracks.rotation, frame) {
            node.rotation = r;
            node.flags &= !eval::UNIT_ANG;
        }
        if let Some(s) = sample_vec(&tracks.scale, frame) {
            node.scale = s;
            node.flags &= !eval::UNIT_SCL;
        }
    }
    *index += 1;
    for c in &mut node.children {
        apply(c, motion, frame, index);
    }
}

/// Character body models start below the animation's first nodes: a motion
/// built for a 62-node skeleton drives a 60-node body whose two root nodes
/// (an empty root and the hip-height offset) live only in the skeleton's own
/// model. Returns `body` wrapped in the first `n` nodes of `skeleton`'s
/// first-child chain (transforms kept, geometry removed).
pub fn with_prefix(body: &Node, skeleton: &Node, n: usize) -> Node {
    if n == 0 {
        return body.clone();
    }
    let mut node = skeleton.clone();
    node.attach = None;
    let rest = skeleton.children.first().map(|c| with_prefix(body, c, n - 1)).unwrap_or_else(|| body.clone());
    node.children = vec![rest];
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ninja::motion::NodeTracks;

    fn leaf() -> Node {
        Node { flags: 0, position: [0.0; 3], rotation: [0; 3], scale: [1.0; 3], attach: None, children: vec![] }
    }

    #[test]
    fn interpolates_position_and_wraps_frames() {
        let mut root = leaf();
        root.children.push(leaf());
        let m = Motion {
            frame_count: 10,
            type_flags: 3,
            nodes: vec![
                NodeTracks { position: vec![Key { frame: 0, value: [0.0, 0.0, 0.0] }, Key { frame: 10, value: [10.0, 0.0, 0.0] }], ..Default::default() },
                NodeTracks { rotation: vec![Key { frame: 0, value: [0xFF00, 0, 0] }, Key { frame: 4, value: [0x0100, 0, 0] }], ..Default::default() },
            ],
        };
        let p = posed(&root, &m, 5.0);
        assert!((p.position[0] - 5.0).abs() < 1e-4);
        // 0xFF00 -> 0x0100 goes the short way (+0x200), halfway at frame 2.
        let p = posed(&root, &m, 2.0);
        assert_eq!(p.children[0].rotation[0] & 0xFFFF, 0x0000);
        // Frame 12 wraps to 2.
        assert_eq!(posed(&root, &m, 12.0).children[0].rotation[0] & 0xFFFF, 0x0000);
    }

    #[test]
    fn prefix_wraps_the_body_and_aligns_motion_indices() {
        let mut skel = leaf();
        skel.position = [0.0, 5.0, 0.0];
        skel.attach = None;
        let mut inner = leaf();
        inner.position = [0.0, 1.0, 0.0];
        skel.children.push(inner);
        let mut body = leaf();
        body.position = [7.0, 0.0, 0.0];
        let wrapped = with_prefix(&body, &skel, 2);
        assert_eq!(wrapped.count(), 3);
        assert_eq!(wrapped.position, [0.0, 5.0, 0.0]);
        assert_eq!(wrapped.children[0].children[0].position, [7.0, 0.0, 0.0]);
        assert_eq!(with_prefix(&body, &skel, 0).position, [7.0, 0.0, 0.0]);
    }

    #[test]
    fn missing_tracks_leave_node_alone() {
        let mut root = leaf();
        root.position = [1.0, 2.0, 3.0];
        let m = Motion { frame_count: 4, type_flags: 3, nodes: vec![NodeTracks::default()] };
        assert_eq!(posed(&root, &m, 1.0).position, [1.0, 2.0, 3.0]);
    }
}
