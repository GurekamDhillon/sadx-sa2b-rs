# Character models, skinning and animation (SA2 PC)

How a character is drawn from the game's files. Everything here was checked
against the real `sonicmdl.prs` / `plcommtn.prs` / `sonicmtn.prs` and the
exe; the checks are tests in `crates/sa_data/tests/sa2_install.rs`.

## Files

- `*mdl.prs`: big-endian table of `(i32 id, u32 offset)` ended by -1; each
  offset is an `NJS_OBJECT` node tree with **chunk** models (`sa_formats::ninja::chunk`).
- `plcommtn.prs` (ids 0–85) and `*mtn.prs` (ids 87+): motions, big-endian
  table of `(u16 id, u16 node count, u32 offset)` ended by 0xFFFF.
- Per-character animation table in `sonic2app.exe` (`sa_data::anim`, e.g.
  Sonic at 0xA17C18, 203 entries of 16 bytes): animation *state* → motion id
  (`anim_num`), model id (`model_num`), mode, next state, blend and speed.
  `model_num` is a model **id**, not an index into the file.

## Drawing the body

1. **Model matrix** (`Sonic_Display`, 0x720090): translate to the position,
   rotate Z, rotate X, then rotate Y by **`0x8000 − heading`**. The half turn
   is why the model faces the opposite way to the physics forward axis
   (`Body::display_rotation`).
2. **Node transform**: `T · Rx · Ry · Rz · S` (Z·X·Y with the `ZXY_ANG` flag),
   angles are BAMS (0x10000 = turn). Ninja space is the right-handed one
   mirrored in Z, so rotations about **X and Y turn the opposite way** to
   right-handed math (Z does not): `rot_x(-x)`, `rot_y(-y)`, `rot_z(z)`.
   - Z sign: both copies of each weighted vertex coincide at rest (median
     0.026 units; 0.27 with the opposite Z sign).
   - X and Y signs: invisible at rest (those angles are tiny) and found from
     a pose. With plain right-handed angles both arms swing inward and the
     hands meet on the centreline (z ≈ 0.1); with the mirrored convention
     they hang outside their shoulders (z +1.5 and −1.4 against shoulders at
     +0.8 and −0.65). Do not test this on animated poses with the
     coincidence check: animated weighted vertices are *meant* to differ.
3. **Extra root nodes**: a 62-node motion drives a skeleton whose first two
   nodes (an empty root and the 5.24-unit hip height) exist only in the
   skeleton model; body models start below them (`pose::with_prefix`).
4. **Skinning** (chunk vertex cache, `flatten.rs`):
   - A node's vertex chunks write cache slots (already transformed by that
     node). Plain chunks set a slot; weighted chunks carry weight in the high
     16 bits of the "ninja flags" word and the slot in the low 16 plus the
     chunk's index offset. Status is the low two bits of the chunk flags:
     Start (0) sets, Middle (1) and End (2) add.
   - A node with vertices *and* polygons is drawn at once.
   - A polygon-only "skin" node is drawn at the first moment **every slot it
     uses has been rewritten since the node was entered and no blend into it
     is still open**. Found by replaying the cache over every model and
     choosing, per skin node, the moment with the least-stretched triangles;
     drawing on entry or after the whole subtree stretches the body (edges up
     to 6.9 units on a 10-unit character, versus 1.8 with this rule).
     Slot numbers are reused by later subtrees (the second arm rewrites the
     first arm's slots), so draw time matters.

## Playing animations

- State numbers follow the headers' `CHARACTER_ANIMATIONS` (0 stand, 6 walk,
  8 jog, 9 run, 10 sprint, 11 spin dash, 30 ball, 65–68 jump, 100 homing).
- Run cycles advance by `anim_speed × ground speed` per frame; other states
  by `anim_speed`. Table modes 4, 6, 9 play once and hold (inferred).

## Known display issues

- Reflection overlay textures (`*_ref*`, e.g. `stx_ref00`) are environment-mapped
  in the game; the demo skips them, so chrome highlights are missing.
- Ninja data is used as-is in a right-handed viewer, so the character is a
  mirror image of the in-game one (asymmetric details such as logos are
  flipped). The skeleton itself is consistent.

## Not done

- Blending between animations (the table's transition speed is read, not used).
- Which animation id SA2 picks in every situation; the demo's choice
  (`sa_demo::pick_state`) is a reasonable mapping, not decoded.
- Basic and GameCube models are not posed by weights (they have none).
