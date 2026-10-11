//! Flattens a node tree into one world-space triangle mesh (rest pose).

use super::basic::{BasicModel, PolyType};
use super::chunk::{ChunkModel, PolyChunk, WeightStatus};
use super::math::Mat4;
use super::object::{eval, Attach, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubMesh {
    /// Index into the model's texture list, if any texture is bound.
    pub texture_id: Option<u32>,
    /// Range in `Mesh::indices`.
    pub first_index: u32,
    pub index_count: u32,
}

/// Triangle list. Every corner has its own vertex (no sharing between strips).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub submeshes: Vec<SubMesh>,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.positions.iter();
        let first = *it.next()?;
        Some(it.fold((first, first), |(mut lo, mut hi), p| {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
            (lo, hi)
        }))
    }

    fn push(&mut self, p: [f32; 3], n: [f32; 3], uv: [f32; 2]) -> u32 {
        self.positions.push(p);
        self.normals.push(n);
        self.uvs.push(uv);
        (self.positions.len() - 1) as u32
    }

    fn begin(&mut self, texture_id: Option<u32>) {
        let first = self.indices.len() as u32;
        match self.submeshes.last_mut() {
            Some(s) if s.texture_id == texture_id && s.first_index + s.index_count == first => {}
            _ => self.submeshes.push(SubMesh { texture_id, first_index: first, index_count: 0 }),
        }
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
        self.submeshes.last_mut().expect("begin() first").index_count += 3;
    }
}

#[derive(Clone, Copy, Default)]
struct CacheVertex {
    pos: [f32; 3],
    normal: [f32; 3],
}

/// Flattens `root` with every node in its rest pose.
pub fn flatten(root: &Node) -> Mesh {
    let mut mesh = Mesh::default();
    let mut cache: Vec<CacheVertex> = Vec::new();
    root.walk(&Mat4::IDENTITY, &mut |node, world| {
        if node.flags & eval::HIDE != 0 {
            return;
        }
        match &node.attach {
            Some(Attach::Chunk(m)) => chunk(&mut mesh, &mut cache, m, world),
            Some(Attach::Basic(m)) => basic(&mut mesh, m, world),
            None => {}
        }
    });
    mesh
}

fn chunk(mesh: &mut Mesh, cache: &mut Vec<CacheVertex>, m: &ChunkModel, world: &Mat4) {
    for vc in &m.vertex_chunks {
        let ws = vc.weight_status();
        for (i, v) in vc.vertices.iter().enumerate() {
            let (slot, weight) = match (ws, v.ninja_flags) {
                (Some(_), Some(f)) => ((f & 0xFFFF) as usize + vc.index_offset as usize, (f >> 16) as f32 / 255.0),
                _ => (vc.index_offset as usize + i, 1.0),
            };
            if slot >= cache.len() {
                cache.resize(slot + 1, CacheVertex::default());
            }
            let p = world.point(v.position);
            let n = world.normal(v.normal.unwrap_or([0., 1., 0.]));
            let e = &mut cache[slot];
            match ws {
                None | Some(WeightStatus::Start) => {
                    e.pos = p.map(|c| c * weight);
                    e.normal = n.map(|c| c * weight);
                }
                Some(_) => {
                    for k in 0..3 {
                        e.pos[k] += p[k] * weight;
                        e.normal[k] += n[k] * weight;
                    }
                }
            }
        }
    }
    let mut texture = None;
    for pc in &m.poly_chunks {
        match pc {
            PolyChunk::Texture { texture_id, .. } => texture = Some(*texture_id as u32),
            PolyChunk::Strip { strips, .. } => {
                mesh.begin(texture);
                for s in strips {
                    let ids: Vec<u32> = s
                        .vertices
                        .iter()
                        .map(|sv| {
                            let c = cache.get(sv.index as usize).copied().unwrap_or_default();
                            mesh.push(c.pos, sv.normal.map(|n| world.normal(n)).unwrap_or(c.normal), sv.uv.unwrap_or([0., 0.]))
                        })
                        .collect();
                    strip_tris(mesh, &ids, s.reversed);
                }
            }
            _ => {}
        }
    }
}

fn strip_tris(mesh: &mut Mesh, ids: &[u32], reversed: bool) {
    for i in 0..ids.len().saturating_sub(2) {
        let (a, b, c) = (ids[i], ids[i + 1], ids[i + 2]);
        if a == b || b == c || a == c {
            continue;
        }
        if (i % 2 == 1) != reversed {
            mesh.tri(b, a, c);
        } else {
            mesh.tri(a, b, c);
        }
    }
}

fn basic(mesh: &mut Mesh, m: &BasicModel, world: &Mat4) {
    for ms in &m.meshsets {
        let texture = m.materials.get(ms.material as usize).map(|mat| mat.texture_id);
        mesh.begin(texture);
        let mut corner = 0usize;
        for (pi, poly) in ms.polys.iter().enumerate() {
            let ids: Vec<u32> = poly
                .iter()
                .enumerate()
                .map(|(k, &vi)| {
                    let p = m.points.get(vi as usize).copied().unwrap_or_default();
                    let n = m.normals.as_ref().and_then(|ns| ns.get(vi as usize).copied()).unwrap_or([0., 1., 0.]);
                    let uv = ms.uvs.as_ref().and_then(|u| u.get(corner + k).copied()).unwrap_or([0., 0.]);
                    mesh.push(world.point(p), world.normal(n), uv)
                })
                .collect();
            corner += poly.len();
            match ms.poly_type {
                PolyType::Triangles => mesh.tri(ids[0], ids[1], ids[2]),
                PolyType::Quads => {
                    mesh.tri(ids[0], ids[1], ids[2]);
                    mesh.tri(ids[2], ids[1], ids[3]);
                }
                PolyType::Strip | PolyType::NStrip => strip_tris(mesh, &ids, ms.reversed.get(pi).copied().unwrap_or(false)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::object::{ModelKind, Node};
    use super::super::{basic, chunk};
    use super::*;
    use crate::bytes::{Bytes, Endian, FileOffsets};

    fn node_at(d: &mut Vec<u8>, attach: usize, pos: [f32; 3], be: bool) -> usize {
        let at = d.len();
        let w32 = |v: u32| if be { v.to_be_bytes() } else { v.to_le_bytes() };
        let wf = |v: f32| if be { v.to_be_bytes() } else { v.to_le_bytes() };
        d.extend_from_slice(&w32(0));
        d.extend_from_slice(&w32(attach as u32));
        for f in pos {
            d.extend_from_slice(&wf(f));
        }
        d.extend_from_slice(&[0; 12]);
        for _ in 0..3 {
            d.extend_from_slice(&wf(1.0));
        }
        d.extend_from_slice(&[0; 8]);
        at
    }

    #[test]
    fn chunk_triangle_translated() {
        let mut d = chunk::tests::triangle_model_be();
        let n = node_at(&mut d, 0x10, [10., 0., 0.], true);
        let root = Node::read(&Bytes::new(&d, Endian::Big), &FileOffsets, n, ModelKind::Chunk).unwrap();
        let mesh = flatten(&root);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.positions[1], [11., 0., 0.]);
        assert_eq!(mesh.uvs[1], [1., 0.]);
        assert_eq!(mesh.submeshes, vec![SubMesh { texture_id: Some(5), first_index: 0, index_count: 3 }]);
    }

    #[test]
    fn basic_quad_two_triangles() {
        let mut d = basic::tests::quad_model_le();
        let n = node_at(&mut d, 0x10, [0., 5., 0.], false);
        let root = Node::read(&Bytes::new(&d, Endian::Little), &FileOffsets, n, ModelKind::Basic(basic::BasicVariant::Sa1)).unwrap();
        let mesh = flatten(&root);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.bounds(), Some(([0., 5., 0.], [1., 5., 1.])));
    }

    #[test]
    fn strip_winding_alternates() {
        let mut m = Mesh::default();
        m.begin(None);
        strip_tris(&mut m, &[0, 1, 2, 3], false);
        assert_eq!(m.indices, vec![0, 1, 2, 2, 1, 3]);
        let mut m = Mesh::default();
        m.begin(None);
        strip_tris(&mut m, &[0, 1, 2], true);
        assert_eq!(m.indices, vec![1, 0, 2]);
    }
}
