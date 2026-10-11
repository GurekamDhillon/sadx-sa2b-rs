//! C ABI for the SA rewrite library. The header `include/sa.h` is generated
//! from this file by cbindgen at build time.
//!
//! Conventions:
//! - Every function returns `SaStatus`; on failure `sa_last_error()` gives a
//!   thread-local, NUL-terminated message valid until the next call.
//! - Objects created by the library are opaque handles, each released by its
//!   matching `_free`/`_close` function. The library never frees memory
//!   owned by the caller.
//! - Panics never cross the boundary; they become `SA_STATUS_INTERNAL`.
//! - Characters and costumes are plain `uint32_t` values (`SA_CHARACTER_*`,
//!   `SA_COSTUME_*`); out-of-range values return `SA_STATUS_RANGE`.

#![allow(clippy::missing_safety_doc)]

pub mod controller;

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use sa_data::catalog::Costume;
use sa_data::chao_save::{Chao, Sa2ChaoFile};
use sa_data::enums::{Animal, ChaoStat, ChaoType, Character};
use sa_data::{Game, GameKind};
use sa_formats::ninja::Mesh;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaStatus {
    Ok = 0,
    NullArgument = 1,
    NotFound = 2,
    Unsupported = 3,
    Format = 4,
    Io = 5,
    NotAvailable = 6,
    Range = 7,
    Internal = 8,
}

pub const SA_GAME_SA2_STEAM: u32 = 0;
pub const SA_GAME_SADX_2004: u32 = 1;

pub const SA_CHARACTER_SONIC: u32 = 0;
pub const SA_CHARACTER_SHADOW: u32 = 1;
pub const SA_CHARACTER_TAILS: u32 = 2;
pub const SA_CHARACTER_EGGMAN: u32 = 3;
pub const SA_CHARACTER_KNUCKLES: u32 = 4;
pub const SA_CHARACTER_ROUGE: u32 = 5;
pub const SA_CHARACTER_MECH_TAILS: u32 = 6;
pub const SA_CHARACTER_MECH_EGGMAN: u32 = 7;
pub const SA_CHARACTER_AMY: u32 = 8;
pub const SA_CHARACTER_SUPER_SONIC: u32 = 9;
pub const SA_CHARACTER_SUPER_SHADOW: u32 = 10;
pub const SA_CHARACTER_METAL_SONIC: u32 = 12;
pub const SA_CHARACTER_CHAO_WALKER: u32 = 13;
pub const SA_CHARACTER_DARK_CHAO_WALKER: u32 = 14;
pub const SA_CHARACTER_TIKAL: u32 = 15;
pub const SA_CHARACTER_CHAOS: u32 = 16;

pub const SA_COSTUME_DEFAULT: u32 = 0;
pub const SA_COSTUME_ALTERNATE: u32 = 1;

pub const SA_CHAO_SLOT_COUNT: u32 = 37;

/// Opaque handle to an opened game install.
pub struct SaGame(Game);
/// Opaque flattened triangle mesh.
pub struct SaMesh(Mesh);
/// Opaque list of decoded RGBA8 textures.
pub struct SaTextureList(Vec<(CString, sa_formats::texture::Rgba)>);
/// Opaque collision triangle soup.
pub struct SaCollision(sa_formats::landtable::CollisionMesh);
/// Opaque Chao save file.
pub struct SaChaoFile(Sa2ChaoFile);

/// Character physics (`PhysicsData`, same layout as the game's 0x84 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SaPhysics {
    pub hang_time: i32,
    pub floor_grip: f32,
    pub speed_cap_h: f32,
    pub speed_cap_v: f32,
    pub speed_max_h: f32,
    pub push_speed_max: f32,
    pub jump_speed: f32,
    pub no_control_speed: f32,
    pub slide_speed: f32,
    pub jog_speed: f32,
    pub run_speed: f32,
    pub rush_speed: f32,
    pub knockback_speed: f32,
    pub dash_speed: f32,
    pub jump_add: f32,
    pub run_accel: f32,
    pub air_accel: f32,
    pub run_decel: f32,
    pub run_brake: f32,
    pub air_brake: f32,
    pub air_decel: f32,
    pub air_resist: f32,
    pub air_resist_v: f32,
    pub air_resist_h: f32,
    pub ground_friction: f32,
    pub ground_friction_h: f32,
    pub friction_cap: f32,
    pub rat_bound: f32,
    pub radius: f32,
    pub height: f32,
    pub weight: f32,
    pub camera_y: f32,
    pub center_height: f32,
}

/// A range of `indices` drawn with one texture (`texture_id` < 0: none).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SaSubMesh {
    pub texture_id: i32,
    pub first_index: u32,
    pub index_count: u32,
}

/// Borrowed view of an `SaMesh`; pointers stay valid until `sa_mesh_free`.
/// `positions`/`normals` hold 3 floats per vertex, `uvs` 2. Use
/// `sa_mesh_submesh` for the texture ranges.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SaMeshView {
    pub vertex_count: u32,
    pub index_count: u32,
    pub submesh_count: u32,
    pub positions: *const f32,
    pub normals: *const f32,
    pub uvs: *const f32,
    pub indices: *const u32,
}

/// Borrowed view of one texture; valid until `sa_textures_free`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SaTextureView {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8, rows top to bottom.
    pub rgba: *const u8,
    pub name: *const c_char,
}

/// Borrowed view of collision geometry; valid until `sa_collision_free`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SaCollisionView {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub positions: *const f32,
    pub indices: *const u32,
    /// Surface flags per triangle (`SurfaceFlags`).
    pub flags: *const u32,
}

/// Copy of the commonly used fields of one Chao slot.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SaChao {
    /// `ChaoType`; 0 = empty slot.
    pub chao_type: u8,
    pub garden: u8,
    /// Game-encoded name, NUL padded (not ASCII).
    pub name: [u8; 7],
    pub happiness: i16,
    pub lifespan: i16,
    pub reincarnations: i16,
    /// Per stat: swim, fly, run, power, stamina, luck, intelligence, unknown.
    pub stat_grades: [u8; 8],
    pub stat_levels: [u8; 8],
    pub stat_points: [u16; 8],
    pub power_run: f32,
    pub fly_swim: f32,
    pub alignment: f32,
    pub evolution_progress: f32,
    pub color: u8,
    pub texture: u8,
    pub shiny: u8,
    pub monotone: u8,
    pub headgear: u8,
    pub medal: u8,
    /// Arm, ear, forehead, horn, leg, tail, wing, face; -1 = none.
    pub animal_parts: [i8; 8],
    pub favorite_fruit: u8,
    pub toys: u16,
    pub classroom_flags: u32,
}

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

fn set_error(msg: impl Into<String>) {
    let s = CString::new(msg.into().replace('\0', " ")).unwrap_or_default();
    LAST_ERROR.with(|e| *e.borrow_mut() = s);
}

pub(crate) type Fail = (SaStatus, String);

/// Runs `f`, converting errors and panics into a status + last error.
pub(crate) fn guard(f: impl FnOnce() -> Result<(), Fail>) -> SaStatus {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => SaStatus::Ok,
        Ok(Err((s, msg))) => {
            set_error(msg);
            s
        }
        Err(p) => {
            let msg = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "panic".into());
            set_error(format!("internal error: {msg}"));
            SaStatus::Internal
        }
    }
}

fn data_err(e: sa_data::Error) -> Fail {
    use sa_data::Error::*;
    let s = match &e {
        Format(_) => SaStatus::Format,
        Io { .. } => SaStatus::Io,
        Unsupported(_) => SaStatus::Unsupported,
        NotFound(_) => SaStatus::NotFound,
        NotAvailable(_) => SaStatus::NotAvailable,
    };
    (s, e.to_string())
}

pub(crate) fn null() -> Fail {
    (SaStatus::NullArgument, "null argument".into())
}

fn range(what: &str, v: impl std::fmt::Display) -> Fail {
    (SaStatus::Range, format!("{what} {v} out of range"))
}

unsafe fn game<'a>(g: *const SaGame) -> Result<&'a Game, Fail> {
    g.as_ref().map(|g| &g.0).ok_or_else(null)
}

fn character(c: u32) -> Result<Character, Fail> {
    Character::from_index(c).ok_or_else(|| range("character", c))
}

fn costume(c: u32) -> Result<Costume, Fail> {
    match c {
        SA_COSTUME_DEFAULT => Ok(Costume::Default),
        SA_COSTUME_ALTERNATE => Ok(Costume::Alternate),
        _ => Err(range("costume", c)),
    }
}

unsafe fn cstr<'a>(s: *const c_char) -> Result<&'a str, Fail> {
    if s.is_null() {
        return Err(null());
    }
    CStr::from_ptr(s).to_str().map_err(|_| (SaStatus::Range, "string is not valid UTF-8".into()))
}

pub(crate) unsafe fn put<T>(out: *mut *mut T, v: T) -> Result<(), Fail> {
    if out.is_null() {
        return Err(null());
    }
    *out = Box::into_raw(Box::new(v));
    Ok(())
}

pub(crate) unsafe fn write<T>(out: *mut T, v: T) -> Result<(), Fail> {
    *out.as_mut().ok_or_else(null)? = v;
    Ok(())
}

pub(crate) unsafe fn free<T>(p: *mut T) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}

/// Message for the last failed call on this thread ("" if none).
#[no_mangle]
pub extern "C" fn sa_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ptr())
}

/// Opens a game install folder (containing `sonic2app.exe` or `sonic.exe`).
#[no_mangle]
pub unsafe extern "C" fn sa_game_open(path: *const c_char, out: *mut *mut SaGame) -> SaStatus {
    guard(|| put(out, SaGame(Game::open(cstr(path)?).map_err(data_err)?)))
}

#[no_mangle]
pub unsafe extern "C" fn sa_game_close(g: *mut SaGame) {
    free(g)
}

/// Writes `SA_GAME_SA2_STEAM` or `SA_GAME_SADX_2004`.
#[no_mangle]
pub unsafe extern "C" fn sa_game_kind(g: *const SaGame, out: *mut u32) -> SaStatus {
    guard(|| {
        let kind = match game(g)?.kind {
            GameKind::Sa2Steam => SA_GAME_SA2_STEAM,
            GameKind::Sadx2004 => SA_GAME_SADX_2004,
        };
        write(out, kind)
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_physics(g: *const SaGame, character_id: u32, out: *mut SaPhysics) -> SaStatus {
    guard(|| {
        let p = game(g)?.physics(character(character_id)?).map_err(data_err)?;
        // SAFETY: SaPhysics and PhysicsData are repr(C) with identical fields
        // (checked by the `physics_layout_matches` test).
        write(out, std::mem::transmute::<sa_data::physics::PhysicsData, SaPhysics>(p))
    })
}

/// Loads a character's main body model, flattened in rest pose.
#[no_mangle]
pub unsafe extern "C" fn sa_mesh_load_character(g: *const SaGame, character_id: u32, costume_id: u32, out: *mut *mut SaMesh) -> SaStatus {
    guard(|| put(out, SaMesh(game(g)?.character_mesh(character(character_id)?, costume(costume_id)?).map_err(data_err)?)))
}

/// Loads the visible geometry of a landtable export (e.g. "objLandTableNeut").
#[no_mangle]
pub unsafe extern "C" fn sa_mesh_load_landtable(g: *const SaGame, export_name: *const c_char, out: *mut *mut SaMesh) -> SaStatus {
    guard(|| put(out, SaMesh(game(g)?.landtable(cstr(export_name)?).map_err(data_err)?.visual_mesh())))
}

#[no_mangle]
pub unsafe extern "C" fn sa_mesh_view(m: *const SaMesh, out: *mut SaMeshView) -> SaStatus {
    guard(|| {
        let m = &m.as_ref().ok_or_else(null)?.0;
        write(
            out,
            SaMeshView {
                vertex_count: m.positions.len() as u32,
                index_count: m.indices.len() as u32,
                submesh_count: m.submeshes.len() as u32,
                positions: m.positions.as_ptr() as *const f32,
                normals: m.normals.as_ptr() as *const f32,
                uvs: m.uvs.as_ptr() as *const f32,
                indices: m.indices.as_ptr(),
            },
        )
    })
}

/// Copies submesh `i` (texture binding + index range).
#[no_mangle]
pub unsafe extern "C" fn sa_mesh_submesh(m: *const SaMesh, i: u32, out: *mut SaSubMesh) -> SaStatus {
    guard(|| {
        let m = &m.as_ref().ok_or_else(null)?.0;
        let s = m.submeshes.get(i as usize).ok_or_else(|| range("submesh", i))?;
        write(out, SaSubMesh { texture_id: s.texture_id.map(|t| t as i32).unwrap_or(-1), first_index: s.first_index, index_count: s.index_count })
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_mesh_free(m: *mut SaMesh) {
    free(m)
}

fn texture_list(t: Vec<(String, sa_formats::texture::Rgba)>) -> SaTextureList {
    SaTextureList(t.into_iter().map(|(n, i)| (CString::new(n).unwrap_or_default(), i)).collect())
}

/// Loads a character's textures in texture-id order.
#[no_mangle]
pub unsafe extern "C" fn sa_textures_load_character(g: *const SaGame, character_id: u32, costume_id: u32, out: *mut *mut SaTextureList) -> SaStatus {
    guard(|| put(out, texture_list(game(g)?.character_textures(character(character_id)?, costume(costume_id)?).map_err(data_err)?)))
}

/// Loads a texture archive by file name (`*.prs` GVM/PVM, or `*.pak` from `PRS/`).
#[no_mangle]
pub unsafe extern "C" fn sa_textures_load_archive(g: *const SaGame, file_name: *const c_char, out: *mut *mut SaTextureList) -> SaStatus {
    guard(|| put(out, texture_list(game(g)?.texture_archive(cstr(file_name)?).map_err(data_err)?)))
}

#[no_mangle]
pub unsafe extern "C" fn sa_textures_count(t: *const SaTextureList, out: *mut u32) -> SaStatus {
    guard(|| write(out, t.as_ref().ok_or_else(null)?.0.len() as u32))
}

#[no_mangle]
pub unsafe extern "C" fn sa_textures_get(t: *const SaTextureList, i: u32, out: *mut SaTextureView) -> SaStatus {
    guard(|| {
        let (name, img) = t.as_ref().ok_or_else(null)?.0.get(i as usize).ok_or_else(|| range("texture", i))?;
        write(out, SaTextureView { width: img.width, height: img.height, rgba: img.pixels.as_ptr(), name: name.as_ptr() })
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_textures_free(t: *mut SaTextureList) {
    free(t)
}

/// Loads solid/water collision triangles of a landtable export.
#[no_mangle]
pub unsafe extern "C" fn sa_collision_load_landtable(g: *const SaGame, export_name: *const c_char, out: *mut *mut SaCollision) -> SaStatus {
    guard(|| put(out, SaCollision(game(g)?.landtable(cstr(export_name)?).map_err(data_err)?.collision_mesh())))
}

#[no_mangle]
pub unsafe extern "C" fn sa_collision_view(c: *const SaCollision, out: *mut SaCollisionView) -> SaStatus {
    guard(|| {
        let c = &c.as_ref().ok_or_else(null)?.0;
        write(
            out,
            SaCollisionView {
                vertex_count: c.positions.len() as u32,
                triangle_count: c.flags.len() as u32,
                positions: c.positions.as_ptr() as *const f32,
                indices: c.indices.as_ptr(),
                flags: c.flags.as_ptr(),
            },
        )
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_collision_free(c: *mut SaCollision) {
    free(c)
}

/// Parses an SA2 Chao save (`SONIC2B__ALF` contents). The bytes are copied.
#[no_mangle]
pub unsafe extern "C" fn sa_chao_file_read(bytes: *const u8, len: usize, out: *mut *mut SaChaoFile) -> SaStatus {
    guard(|| {
        if bytes.is_null() {
            return Err(null());
        }
        put(out, SaChaoFile(Sa2ChaoFile::read(std::slice::from_raw_parts(bytes, len)).map_err(data_err)?))
    })
}

/// Number of slots present in the file (at most `SA_CHAO_SLOT_COUNT`).
#[no_mangle]
pub unsafe extern "C" fn sa_chao_file_slot_count(f: *const SaChaoFile, out: *mut u32) -> SaStatus {
    guard(|| write(out, f.as_ref().ok_or_else(null)?.0.slots.len() as u32))
}

fn summarize(c: &Chao) -> SaChao {
    let s = ChaoStat::from_index;
    SaChao {
        chao_type: c.raw[sa_data::chao_save::off::TYPE],
        garden: c.garden(),
        name: c.name_bytes().try_into().unwrap(),
        happiness: c.happiness(),
        lifespan: c.lifespan(),
        reincarnations: c.reincarnations(),
        stat_grades: std::array::from_fn(|i| c.stat_grade(s(i))),
        stat_levels: std::array::from_fn(|i| c.stat_level(s(i))),
        stat_points: std::array::from_fn(|i| c.stat_points(s(i))),
        power_run: c.power_run(),
        fly_swim: c.fly_swim(),
        alignment: c.alignment(),
        evolution_progress: c.evolution_progress(),
        color: c.color(),
        texture: c.texture(),
        shiny: c.shiny() as u8,
        monotone: c.monotone() as u8,
        headgear: c.headgear(),
        medal: c.medal(),
        animal_parts: c.animal_parts().map(|a| a.unwrap_or(Animal::None) as i8),
        favorite_fruit: c.favorite_fruit(),
        toys: c.toys(),
        classroom_flags: c.classroom_flags(),
    }
}

unsafe fn slot<'a>(f: *mut SaChaoFile, i: u32) -> Result<&'a mut Chao, Fail> {
    f.as_mut().ok_or_else(null)?.0.slots.get_mut(i as usize).ok_or_else(|| range("slot", i))
}

#[no_mangle]
pub unsafe extern "C" fn sa_chao_get(f: *const SaChaoFile, slot_index: u32, out: *mut SaChao) -> SaStatus {
    guard(|| {
        let c = f.as_ref().ok_or_else(null)?.0.slots.get(slot_index as usize).ok_or_else(|| range("slot", slot_index))?;
        write(out, summarize(c))
    })
}

/// Sets one stat (0..7 = swim, fly, run, power, stamina, luck, intelligence, unknown).
#[no_mangle]
pub unsafe extern "C" fn sa_chao_set_stat(f: *mut SaChaoFile, slot_index: u32, stat: u32, grade: u8, level: u8, points: u16) -> SaStatus {
    guard(|| {
        if stat > 7 {
            return Err(range("stat", stat));
        }
        slot(f, slot_index)?.set_stat(ChaoStat::from_index(stat as usize), grade, level, points);
        Ok(())
    })
}

/// Sets the hero (+1) / dark (-1) alignment of a slot.
#[no_mangle]
pub unsafe extern "C" fn sa_chao_set_alignment(f: *mut SaChaoFile, slot_index: u32, alignment: f32) -> SaStatus {
    guard(|| {
        slot(f, slot_index)?.set_alignment(alignment);
        Ok(())
    })
}

/// Sets the `ChaoType` byte of a slot (0..=0x19).
#[no_mangle]
pub unsafe extern "C" fn sa_chao_set_type(f: *mut SaChaoFile, slot_index: u32, chao_type: u8) -> SaStatus {
    guard(|| {
        let t = ChaoType::from_u8(chao_type).ok_or_else(|| range("chao type", chao_type))?;
        slot(f, slot_index)?.set_chao_type(t);
        Ok(())
    })
}

/// Serializes the file. Release `*out_bytes` with `sa_bytes_free`.
#[no_mangle]
pub unsafe extern "C" fn sa_chao_file_write(f: *const SaChaoFile, out_bytes: *mut *mut u8, out_len: *mut usize) -> SaStatus {
    guard(|| {
        let f = &f.as_ref().ok_or_else(null)?.0;
        if out_bytes.is_null() || out_len.is_null() {
            return Err(null());
        }
        let b = f.write().into_boxed_slice();
        *out_len = b.len();
        *out_bytes = Box::into_raw(b) as *mut u8;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn sa_bytes_free(bytes: *mut u8, len: usize) {
    if !bytes.is_null() {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(bytes, len)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn sa_chao_file_free(f: *mut SaChaoFile) {
    free(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_layout_matches() {
        assert_eq!(std::mem::size_of::<SaPhysics>(), std::mem::size_of::<sa_data::physics::PhysicsData>());
        assert_eq!(std::mem::size_of::<SaPhysics>(), 0x84);
    }

    #[test]
    fn null_and_range_errors_set_message() {
        unsafe {
            let mut g = ptr::null_mut();
            assert_eq!(sa_game_open(ptr::null(), &mut g), SaStatus::NullArgument);
            assert_eq!(CStr::from_ptr(sa_last_error()).to_str().unwrap(), "null argument");
            let missing = CString::new("Z:/definitely/not/here").unwrap();
            assert_eq!(sa_game_open(missing.as_ptr(), &mut g), SaStatus::NotFound);
            assert!(g.is_null());
            let mut p = SaPhysics::default();
            assert_eq!(sa_physics(ptr::null(), 0, &mut p), SaStatus::NullArgument);
            sa_game_close(ptr::null_mut());
            sa_mesh_free(ptr::null_mut());
        }
    }

    #[test]
    fn chao_file_round_trip_through_c_api() {
        let size = sa_data::chao_save::SA2_SLOT_OFFSET + 2 * sa_data::chao_save::SLOT_SIZE;
        let bytes = vec![0u8; size];
        unsafe {
            let mut f = ptr::null_mut();
            assert_eq!(sa_chao_file_read(bytes.as_ptr(), bytes.len(), &mut f), SaStatus::Ok);
            let mut n = 0;
            assert_eq!(sa_chao_file_slot_count(f, &mut n), SaStatus::Ok);
            assert_eq!(n, 2);
            assert_eq!(sa_chao_set_type(f, 1, ChaoType::Child as u8), SaStatus::Ok);
            assert_eq!(sa_chao_set_stat(f, 1, 2, 4, 50, 1500), SaStatus::Ok);
            assert_eq!(sa_chao_set_stat(f, 1, 8, 0, 0, 0), SaStatus::Range);
            assert_eq!(sa_chao_set_type(f, 5, 1), SaStatus::Range);
            assert_eq!(sa_chao_set_type(f, 1, 0x40), SaStatus::Range);
            let mut c = SaChao::default();
            assert_eq!(sa_chao_get(f, 1, &mut c), SaStatus::Ok);
            assert_eq!(c.chao_type, ChaoType::Child as u8);
            assert_eq!((c.stat_grades[2], c.stat_levels[2], c.stat_points[2]), (4, 50, 1500));
            let (mut out, mut len) = (ptr::null_mut(), 0usize);
            assert_eq!(sa_chao_file_write(f, &mut out, &mut len), SaStatus::Ok);
            assert_eq!(len, size);
            assert_eq!(*out.add(sa_data::chao_save::SA2_SLOT_OFFSET + 0x800 + 0x80), ChaoType::Child as u8);
            sa_bytes_free(out, len);
            sa_chao_file_free(f);
        }
    }

    #[test]
    fn panics_are_caught() {
        assert_eq!(guard(|| panic!("boom")), SaStatus::Internal);
        let msg = unsafe { CStr::from_ptr(sa_last_error()) }.to_str().unwrap().to_owned();
        assert!(msg.contains("boom"), "{msg}");
    }
}
