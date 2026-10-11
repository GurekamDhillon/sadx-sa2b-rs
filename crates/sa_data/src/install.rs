//! Locating and identifying a game install, and loading content from it.

use std::path::{Path, PathBuf};

use sa_formats::bytes::{AddressMap, Endian};
use sa_formats::landtable::LandTable;
use sa_formats::ninja::{self, motion::IndexedMotion, IndexedModel};
use sa_formats::pe::Pe;
use sa_formats::texture::{archive, Rgba};
use sa_formats::{pak, prs};
use sha2::{Digest, Sha256};

use crate::catalog::{self, Costume};
use crate::enums::Character;
use crate::error::{read, Error, Result};
use crate::physics::{self, PhysicsData};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameKind {
    /// Sonic Adventure 2, Steam PC release (`sonic2app.exe`).
    Sa2Steam = 0,
    /// Sonic Adventure DX, 2004 US PC release (`sonic.exe`).
    Sadx2004 = 1,
}

/// SHA-256 of the supported SA2 Steam `sonic2app.exe`.
pub const SA2_STEAM_SHA256: &str = "4b1e09d782a2d919cbed2324ba2c0c78ef5fbe1677d400edab39608191f12c16";
/// SHA-256 of the 2011 SADX Steam `Sonic Adventure DX.exe` (unsupported).
pub const SADX_STEAM_2011_SHA256: &str = "e4330c00d7ee3910ecc1acab789ffd2a2d571c506984b99a8de4d3663e697fd6";

#[derive(Debug, Clone)]
pub struct Game {
    pub root: PathBuf,
    pub kind: GameKind,
    pub exe_sha256: String,
    exe: PathBuf,
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

impl Game {
    /// Opens an install directory (the folder with the game exe).
    pub fn open(root: impl AsRef<Path>) -> Result<Game> {
        let root = root.as_ref().to_path_buf();
        let sa2 = root.join("sonic2app.exe");
        let sadx = root.join("sonic.exe");
        let sadx_steam = root.join("Sonic Adventure DX.exe");
        if sa2.is_file() {
            let hash = sha256_hex(&read(&sa2)?);
            if hash != SA2_STEAM_SHA256 {
                return Err(Error::Unsupported(format!("sonic2app.exe with SHA-256 {hash}; only the Steam build {SA2_STEAM_SHA256} is supported")));
            }
            return Ok(Game { root, kind: GameKind::Sa2Steam, exe_sha256: hash, exe: sa2 });
        }
        if sadx.is_file() {
            // The 2004 exe hash is not pinned yet: identify by name and PE32 image.
            let data = read(&sadx)?;
            Pe::parse(&data)?;
            return Ok(Game { root, kind: GameKind::Sadx2004, exe_sha256: sha256_hex(&data), exe: sadx });
        }
        if sadx_steam.is_file() {
            return Err(Error::Unsupported(
                "this is the 2011 Steam port of SADX; convert it to the 2004 US version with SA Mod Manager (https://github.com/X-Hax/SA-Mod-Manager)".into(),
            ));
        }
        Err(Error::NotFound(format!("no sonic2app.exe or sonic.exe in {}", root.display())))
    }

    fn require_sa2(&self, what: &str) -> Result<()> {
        if self.kind == GameKind::Sa2Steam {
            Ok(())
        } else {
            Err(Error::NotAvailable(format!("{what} is only implemented for SA2")))
        }
    }

    /// `resource/gd_PC` for SA2.
    pub fn data_dir(&self) -> PathBuf {
        match self.kind {
            GameKind::Sa2Steam => self.root.join("resource").join("gd_PC"),
            GameKind::Sadx2004 => self.root.join("system"),
        }
    }

    /// Reads a data file, PRS-decompressing `.prs` files.
    pub fn read_data(&self, name: &str) -> Result<Vec<u8>> {
        let path = self.data_dir().join(name);
        let data = read(&path)?;
        if name.to_ascii_lowercase().ends_with(".prs") {
            Ok(prs::decompress(&data)?)
        } else {
            Ok(data)
        }
    }

    /// All 19 physics rows (SA2), indexed by `Character`.
    pub fn physics_table(&self) -> Result<Vec<PhysicsData>> {
        self.require_sa2("physics")?;
        physics::read_table(&read(&self.exe)?, physics::SA2_PHYSICS_ARRAY, physics::SA2_PHYSICS_COUNT)
    }

    pub fn physics(&self, c: Character) -> Result<PhysicsData> {
        Ok(self.physics_table()?[c as usize])
    }

    fn files(&self, c: Character, costume: Costume) -> Result<catalog::CharacterFiles> {
        self.require_sa2("character assets")?;
        catalog::sa2_character(c, costume).ok_or_else(|| Error::NotFound(format!("no {costume:?} model for {c:?}")))
    }

    /// Every model in the character's model file (id 1 is the main body).
    pub fn character_models(&self, c: Character, costume: Costume) -> Result<Vec<IndexedModel>> {
        let f = self.files(c, costume)?;
        Ok(ninja::read_model_file(&self.read_data(&f.model_file())?, Endian::Big)?)
    }

    /// The character's main body model, flattened in rest pose.
    pub fn character_mesh(&self, c: Character, costume: Costume) -> Result<ninja::Mesh> {
        let models = self.character_models(c, costume)?;
        let main = models.iter().find(|m| m.id == 1).or(models.first()).ok_or_else(|| Error::NotFound(format!("{c:?} model file is empty")))?;
        Ok(ninja::flatten(&main.root))
    }

    pub fn character_motions(&self, c: Character) -> Result<Vec<IndexedMotion>> {
        let f = self.files(c, Costume::Default)?;
        let name = f.motion_file().ok_or_else(|| Error::NotFound(format!("{c:?} has no motion file")))?;
        Ok(ninja::motion::read_motion_file(&self.read_data(&name)?, Endian::Big)?)
    }

    /// Decoded textures in texture-list order (the order texture ids refer to).
    pub fn character_textures(&self, c: Character, costume: Costume) -> Result<Vec<(String, Rgba)>> {
        let f = self.files(c, costume)?;
        self.texture_archive(&f.texture_file())
    }

    /// Decodes a GVM/PVM archive (`*.prs` in the data dir) or a PC `.pak`
    /// (in `PRS/`). PAK textures are returned in archive order, skipping the
    /// `.inf` index.
    pub fn texture_archive(&self, name: &str) -> Result<Vec<(String, Rgba)>> {
        if name.to_ascii_lowercase().ends_with(".pak") {
            let data = read(&self.data_dir().join("PRS").join(name))?;
            return pak::parse(&data)?
                .into_iter()
                .filter(|e| e.name.to_ascii_lowercase().ends_with(".dds"))
                .map(|e| Ok((e.name.clone(), sa_formats::texture::dds::decode(&e.data)?)))
                .collect();
        }
        let a = archive::parse(&self.read_data(name)?)?;
        a.textures.iter().map(|t| Ok((t.name.clone(), t.decode(a.kind)?))).collect()
    }

    /// Reads a landtable exported from `Data_DLL.dll` (SA2).
    pub fn landtable(&self, export: &str) -> Result<LandTable> {
        self.require_sa2("landtables")?;
        let data = read(&self.data_dir().join("DLL").join("Win32").join("Data_DLL.dll"))?;
        let pe = Pe::parse(&data)?;
        let va = pe.export(export).ok_or_else(|| Error::NotFound(format!("export {export} in Data_DLL.dll")))?;
        let off = pe.offset(va).ok_or_else(|| Error::NotFound(format!("{export} address not mapped")))?;
        Ok(LandTable::read_sa2(&data, &pe, off)?)
    }

    /// Path of the main Chao save (`SONIC2B__ALF`), whether or not it exists.
    pub fn chao_save_path(&self) -> PathBuf {
        self.data_dir().join("SAVEDATA").join("SONIC2B__ALF")
    }
}

/// Default Steam library locations to probe for an install.
pub fn steam_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for drive in ['C', 'D', 'E', 'F'] {
        for lib in ["Program Files (x86)/Steam/steamapps/common", "SteamLibrary/steamapps/common"] {
            for game in ["Sonic Adventure 2", "Sonic Adventure DX"] {
                let p = PathBuf::from(format!("{drive}:/{lib}/{game}"));
                if p.is_dir() {
                    out.push(p);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sa_data_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn empty_dir_is_not_found() {
        assert!(matches!(Game::open(temp_dir("empty")), Err(Error::NotFound(_))));
    }

    #[test]
    fn wrong_sa2_hash_is_unsupported() {
        let d = temp_dir("badhash");
        std::fs::write(d.join("sonic2app.exe"), b"not the real exe").unwrap();
        assert!(matches!(Game::open(&d), Err(Error::Unsupported(_))));
    }

    #[test]
    fn sadx_steam_port_is_unsupported_with_hint() {
        let d = temp_dir("sadxsteam");
        std::fs::write(d.join("Sonic Adventure DX.exe"), b"x").unwrap();
        match Game::open(&d) {
            Err(Error::Unsupported(msg)) => assert!(msg.contains("SA Mod Manager")),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}
