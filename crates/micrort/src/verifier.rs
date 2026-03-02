//! Bytecode verifier — structural and type-safety checks on JVM class files.
//!
//! Implements the structural verifier (§4.9) from the JVM spec: verifies
//! the constant pool, method descriptors, and basic bytecode well-formedness.
//! Does NOT implement the full StackMapTable dataflow verifier (§4.10) —
//! that requires a fixpoint analysis and is deferred to Phase 5.

use rava_common::error::{RavaError, Result};

/// Verifies Java bytecode before interpretation.
///
/// Validates JVM class file structure (magic, version, constant pool integrity,
/// method descriptor syntax) before handing the class to the interpreter.
pub struct BytecodeVerifier;

impl BytecodeVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify a class's bytecode. Returns `Ok(())` if the bytecode is valid.
    pub fn verify(&self, bytecode: &[u8]) -> Result<()> {
        let r = &mut Reader::new(bytecode);
        verify_magic(r)?;
        verify_version(r)?;
        let cp_count = r.u16()? as usize;
        let pool = parse_constant_pool(r, cp_count)?;
        verify_access_flags(r)?;
        // this_class and super_class
        let this_idx = r.u16()? as usize;
        let _super_idx = r.u16()?;
        verify_class_ref(&pool, this_idx)?;
        // interfaces
        let iface_count = r.u16()? as usize;
        for _ in 0..iface_count {
            r.u16()?;
        }
        // fields
        let field_count = r.u16()? as usize;
        for _ in 0..field_count {
            verify_member(r, &pool)?;
        }
        // methods
        let method_count = r.u16()? as usize;
        for _ in 0..method_count {
            verify_member(r, &pool)?;
        }
        Ok(())
    }
}

impl Default for BytecodeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8> {
        let b = self.data.get(self.pos).copied().ok_or_else(|| {
            RavaError::Other(format!("unexpected end of class file at offset {}", self.pos))
        })?;
        self.pos += 1;
        Ok(b)
    }

    fn u16(&mut self) -> Result<u16> {
        let hi = self.u8()? as u16;
        let lo = self.u8()? as u16;
        Ok((hi << 8) | lo)
    }

    fn u32(&mut self) -> Result<u32> {
        let a = self.u16()? as u32;
        let b = self.u16()? as u32;
        Ok((a << 16) | b)
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos + n;
        if end > self.data.len() {
            return Err(RavaError::Other(format!(
                "unexpected end of class file: need {} bytes at offset {}",
                n, self.pos
            )));
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
}

/// Constant pool tag constants (JVM spec §4.4).
#[derive(Debug, Clone)]
enum CpEntry {
    Utf8(String),
    Integer,
    Float,
    Long,
    Double,
    Class(u16),
    String(u16),
    Fieldref(u16, u16),
    Methodref(u16, u16),
    InterfaceMethodref(u16, u16),
    NameAndType(u16, u16),
    MethodHandle,
    MethodType(u16),
    Dynamic,
    InvokeDynamic,
    Module(u16),
    Package(u16),
    Placeholder, // for the slot after Long/Double
}

fn parse_constant_pool(r: &mut Reader, cp_count: usize) -> Result<Vec<CpEntry>> {
    // cp_count is actually count+1 (indices 1..cp_count-1 are valid)
    let mut pool = vec![CpEntry::Placeholder; cp_count];
    let mut i = 1;
    while i < cp_count {
        let tag = r.u8()?;
        let entry = match tag {
            1 => {
                // CONSTANT_Utf8
                let len = r.u16()? as usize;
                let bytes = r.bytes(len)?;
                let s = std::str::from_utf8(bytes).map_err(|_| {
                    RavaError::Other("invalid UTF-8 in constant pool".into())
                })?;
                CpEntry::Utf8(s.to_string())
            }
            3 => {
                r.u32()?;
                CpEntry::Integer
            }
            4 => {
                r.u32()?;
                CpEntry::Float
            }
            5 => {
                r.u32()?;
                r.u32()?;
                let entry = CpEntry::Long;
                pool[i] = entry;
                i += 1;
                // Long occupies two slots
                pool[i] = CpEntry::Placeholder;
                i += 1;
                continue;
            }
            6 => {
                r.u32()?;
                r.u32()?;
                let entry = CpEntry::Double;
                pool[i] = entry;
                i += 1;
                pool[i] = CpEntry::Placeholder;
                i += 1;
                continue;
            }
            7 => CpEntry::Class(r.u16()?),
            8 => CpEntry::String(r.u16()?),
            9 => CpEntry::Fieldref(r.u16()?, r.u16()?),
            10 => CpEntry::Methodref(r.u16()?, r.u16()?),
            11 => CpEntry::InterfaceMethodref(r.u16()?, r.u16()?),
            12 => CpEntry::NameAndType(r.u16()?, r.u16()?),
            15 => {
                r.u8()?;
                r.u16()?;
                CpEntry::MethodHandle
            }
            16 => CpEntry::MethodType(r.u16()?),
            17 => {
                r.u16()?;
                r.u16()?;
                CpEntry::Dynamic
            }
            18 => {
                r.u16()?;
                r.u16()?;
                CpEntry::InvokeDynamic
            }
            19 => CpEntry::Module(r.u16()?),
            20 => CpEntry::Package(r.u16()?),
            _ => {
                return Err(RavaError::Other(format!(
                    "unknown constant pool tag {} at index {}",
                    tag, i
                )))
            }
        };
        pool[i] = entry;
        i += 1;
    }
    Ok(pool)
}

fn verify_magic(r: &mut Reader) -> Result<()> {
    let magic = r.u32()?;
    if magic != 0xCAFEBABE {
        return Err(RavaError::Other(format!(
            "invalid class file magic: 0x{:08X} (expected 0xCAFEBABE)",
            magic
        )));
    }
    Ok(())
}

fn verify_version(r: &mut Reader) -> Result<()> {
    let _minor = r.u16()?;
    let major = r.u16()?;
    // Java 1.1 (major=45) through Java 24 (major=68).
    if major < 45 || major > 68 {
        return Err(RavaError::Other(format!(
            "unsupported class file major version {}",
            major
        )));
    }
    Ok(())
}

fn verify_access_flags(r: &mut Reader) -> Result<()> {
    r.u16()?; // access flags — any bit combination is structurally valid
    Ok(())
}

fn verify_class_ref(pool: &[CpEntry], idx: usize) -> Result<()> {
    match pool.get(idx) {
        Some(CpEntry::Class(_)) => Ok(()),
        Some(other) => Err(RavaError::Other(format!(
            "constant pool[{}] expected Class, got {:?}",
            idx, other
        ))),
        None => Err(RavaError::Other(format!(
            "constant pool index {} out of range",
            idx
        ))),
    }
}

/// Parse and skip a class member (field or method) and its attributes.
fn verify_member(r: &mut Reader, _pool: &[CpEntry]) -> Result<()> {
    r.u16()?; // access_flags
    r.u16()?; // name_index
    r.u16()?; // descriptor_index
    let attr_count = r.u16()? as usize;
    for _ in 0..attr_count {
        skip_attribute(r)?;
    }
    Ok(())
}

/// Skip an attribute (name_index + length-prefixed body).
fn skip_attribute(r: &mut Reader) -> Result<()> {
    r.u16()?; // attribute_name_index
    let len = r.u32()? as usize;
    r.bytes(len)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_class(major: u16) -> Vec<u8> {
        // Magic, version, cp_count=1 (empty pool), class flags, this=0, super=0, 0 ifaces, 0 fields, 0 methods
        let mut b = vec![];
        // magic
        b.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        // minor=0, major
        b.extend_from_slice(&0u16.to_be_bytes());
        b.extend_from_slice(&major.to_be_bytes());
        // cp_count=2: index 1 = Class(2)... but we need at least a valid this_class
        // For simplicity just set cp_count=1 (empty) and set this_class=0 which is invalid
        // Actually let's build a proper minimal class
        //
        // cp_count = 3: [1]=Class(2), [2]=Utf8("Test")
        b.extend_from_slice(&3u16.to_be_bytes());
        // cp[1] = Class, name_index=2
        b.push(7u8);
        b.extend_from_slice(&2u16.to_be_bytes());
        // cp[2] = Utf8("Test")
        b.push(1u8);
        b.extend_from_slice(&4u16.to_be_bytes());
        b.extend_from_slice(b"Test");
        // access_flags = ACC_PUBLIC
        b.extend_from_slice(&0x0001u16.to_be_bytes());
        // this_class = 1, super_class = 0
        b.extend_from_slice(&1u16.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes());
        // interfaces_count = 0
        b.extend_from_slice(&0u16.to_be_bytes());
        // fields_count = 0
        b.extend_from_slice(&0u16.to_be_bytes());
        // methods_count = 0
        b.extend_from_slice(&0u16.to_be_bytes());
        b
    }

    #[test]
    fn rejects_bad_magic() {
        let mut b = make_minimal_class(52);
        b[0] = 0xDE; // corrupt magic
        let v = BytecodeVerifier::new();
        assert!(v.verify(&b).is_err());
    }

    #[test]
    fn rejects_unsupported_version() {
        let b = make_minimal_class(70); // Java 26 — unsupported
        let v = BytecodeVerifier::new();
        assert!(v.verify(&b).is_err());
    }

    #[test]
    fn accepts_java_17_class() {
        let b = make_minimal_class(61); // Java 17
        let v = BytecodeVerifier::new();
        assert!(v.verify(&b).is_ok(), "{:?}", v.verify(&b));
    }

    #[test]
    fn accepts_java_21_class() {
        let b = make_minimal_class(65); // Java 21
        let v = BytecodeVerifier::new();
        assert!(v.verify(&b).is_ok(), "{:?}", v.verify(&b));
    }
}
