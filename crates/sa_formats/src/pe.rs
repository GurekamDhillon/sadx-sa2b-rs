//! Minimal 32-bit PE reader: section table and virtual-address mapping.
//!
//! Used to read data tables (physics, landtables) straight out of
//! `sonic2app.exe`, `sonic.exe` or `Data_DLL.dll` files on disk.

use crate::bytes::{AddressMap, Bytes, Endian};
use crate::error::{invalid, Error, Result};

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_offset: u32,
    pub raw_size: u32,
}

#[derive(Debug, Clone)]
pub struct Pe {
    pub image_base: u32,
    pub sections: Vec<Section>,
    /// Exported symbol name → virtual address (absolute).
    pub exports: Vec<(String, u32)>,
}

impl Pe {
    pub fn parse(data: &[u8]) -> Result<Pe> {
        let b = Bytes::new(data, Endian::Little);
        if b.slice(0, 2)? != b"MZ" {
            return Err(Error::BadMagic { expected: "MZ", found: b.slice(0, 2)?.to_vec() });
        }
        let pe = b.u32(0x3C)? as usize;
        if b.slice(pe, 4)? != b"PE\0\0" {
            return Err(Error::BadMagic { expected: "PE\\0\\0", found: b.slice(pe, 4)?.to_vec() });
        }
        let nsec = b.u16(pe + 6)? as usize;
        let opt_size = b.u16(pe + 20)? as usize;
        let opt = pe + 24;
        if b.u16(opt)? != 0x10B {
            return Err(invalid(opt, "not a PE32 (32-bit) image"));
        }
        let image_base = b.u32(opt + 28)?;
        let sec_table = opt + opt_size;
        let mut sections = Vec::with_capacity(nsec);
        for i in 0..nsec {
            let s = sec_table + i * 40;
            sections.push(Section {
                name: b.cstr(s, 8)?,
                virtual_size: b.u32(s + 8)?,
                virtual_address: b.u32(s + 12)?,
                raw_size: b.u32(s + 16)?,
                raw_offset: b.u32(s + 20)?,
            });
        }
        let mut pe_out = Pe { image_base, sections, exports: Vec::new() };
        let ndirs = b.u32(opt + 92)?;
        if ndirs > 0 {
            let export_rva = b.u32(opt + 96)?;
            if export_rva != 0 {
                pe_out.exports = pe_out.read_exports(&b, export_rva)?;
            }
        }
        Ok(pe_out)
    }

    fn rva_offset(&self, rva: u32) -> Option<usize> {
        self.sections.iter().find_map(|s| {
            let size = s.virtual_size.max(s.raw_size);
            (rva >= s.virtual_address && rva < s.virtual_address + size)
                .then(|| rva - s.virtual_address)
                .filter(|d| *d < s.raw_size)
                .map(|d| (s.raw_offset + d) as usize)
        })
    }

    fn read_exports(&self, b: &Bytes, rva: u32) -> Result<Vec<(String, u32)>> {
        let dir = self.rva_offset(rva).ok_or_else(|| invalid(0, "export directory unmapped"))?;
        let nnames = b.u32(dir + 24)? as usize;
        let funcs = self.rva_offset(b.u32(dir + 28)?).ok_or_else(|| invalid(dir, "export table unmapped"))?;
        let names = self.rva_offset(b.u32(dir + 32)?).ok_or_else(|| invalid(dir, "name table unmapped"))?;
        let ords = self.rva_offset(b.u32(dir + 36)?).ok_or_else(|| invalid(dir, "ordinal table unmapped"))?;
        let mut out = Vec::with_capacity(nnames);
        for i in 0..nnames {
            let name_off = self.rva_offset(b.u32(names + i * 4)?).ok_or_else(|| invalid(names, "name unmapped"))?;
            let ord = b.u16(ords + i * 2)? as usize;
            let func_rva = b.u32(funcs + ord * 4)?;
            out.push((b.cstr(name_off, 256)?, self.image_base.wrapping_add(func_rva)));
        }
        Ok(out)
    }

    pub fn export(&self, name: &str) -> Option<u32> {
        self.exports.iter().find(|(n, _)| n == name).map(|(_, a)| *a)
    }
}

impl AddressMap for Pe {
    /// Maps an absolute virtual address (as used by the mod loader headers).
    fn offset(&self, addr: u32) -> Option<usize> {
        if addr == 0 {
            return None;
        }
        self.rva_offset(addr.checked_sub(self.image_base)?)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a tiny PE32 with one section `.data` at RVA 0x1000, raw 0x200.
    pub fn tiny_pe(payload: &[u8]) -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[0..2].copy_from_slice(b"MZ");
        d[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        d[0x40..0x44].copy_from_slice(b"PE\0\0");
        d[0x46..0x48].copy_from_slice(&1u16.to_le_bytes()); // sections
        d[0x54..0x56].copy_from_slice(&0xE0u16.to_le_bytes()); // opt header size
        let opt = 0x58;
        d[opt..opt + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        d[opt + 28..opt + 32].copy_from_slice(&0x400000u32.to_le_bytes());
        let sec = opt + 0xE0;
        d[sec..sec + 5].copy_from_slice(b".data");
        d[sec + 8..sec + 12].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        d[sec + 12..sec + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        d[sec + 16..sec + 20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        d[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes());
        d.extend_from_slice(payload);
        d
    }

    #[test]
    fn maps_virtual_addresses() {
        let pe_bytes = tiny_pe(&[0xAA; 0x20]);
        let pe = Pe::parse(&pe_bytes).unwrap();
        assert_eq!(pe.image_base, 0x400000);
        assert_eq!(pe.offset(0x401000), Some(0x200));
        assert_eq!(pe.offset(0x40101F), Some(0x21F));
        assert_eq!(pe.offset(0x401020), None);
        assert_eq!(pe.offset(0x300000), None);
        assert_eq!(pe.offset(0), None);
    }

    #[test]
    fn rejects_non_pe() {
        assert!(matches!(Pe::parse(b"ZZ\0\0"), Err(Error::BadMagic { .. })));
        assert!(matches!(Pe::parse(b"M"), Err(Error::Eof(_))));
    }
}
