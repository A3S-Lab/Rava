//! Minimal JVM `.class` file parser — extracts the class name, superclass, and each
//! method's name, descriptor, access flags, and Code-attribute bytecode.
//!
//! This is the front door for executing **compiled** Java (JARs / dependencies): a
//! later pass lowers method bytecode to RIR so the existing RIR interpreter can run
//! it, instead of maintaining a separate bytecode interpreter.
// ponytail: the ~40-line `Reader` is duplicated from verifier.rs on purpose — unifying
// would mean refactoring verifier's 22 passing tests for no behavior gain. Merge if a
// third consumer appears.

use rava_common::error::{RavaError, Result};

/// A parsed `.class` file (only the parts needed to run a method).
#[derive(Clone)]
pub struct ClassFile {
    pub name: String,
    pub super_name: Option<String>,
    /// Declared instance/static field names (used to populate the interpreter's
    /// field-id → name table so `getfield`/`putfield` resolve).
    pub fields: Vec<String>,
    pub methods: Vec<Method>,
    pool: ConstantPool,
}

impl ClassFile {
    /// Resolve a `Methodref` constant-pool index (an `invoke*` operand) to its
    /// (class name, method name, descriptor).
    pub fn method_ref(&self, index: u16) -> Result<(String, String, String)> {
        self.pool.method_ref(index)
    }

    /// Resolve a `Fieldref` index (a `getfield`/`putfield` operand). Shares the
    /// class + NameAndType shape with method refs.
    pub fn field_ref(&self, index: u16) -> Result<(String, String, String)> {
        self.pool.method_ref(index)
    }

    /// Resolve a `Class` constant-pool index → its name (a `new` operand).
    pub fn class_name_at(&self, index: u16) -> Result<String> {
        self.pool.class_name(index)
    }
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub descriptor: String,
    pub is_static: bool,
    /// Raw Code-attribute bytecode (`None` for abstract / native methods).
    pub code: Option<Vec<u8>>,
}

/// Parse the bytes of a `.class` file.
pub fn parse(bytes: &[u8]) -> Result<ClassFile> {
    let r = &mut Reader::new(bytes);
    if r.u32()? != 0xCAFE_BABE {
        return Err(RavaError::Other("not a Java class file (bad magic)".into()));
    }
    let _minor = r.u16()?;
    let _major = r.u16()?;
    let cp = parse_constant_pool(r)?;
    let _access_flags = r.u16()?;
    let this_class = r.u16()?;
    let super_class = r.u16()?;
    let name = cp.class_name(this_class)?;
    let super_name = if super_class == 0 {
        None
    } else {
        cp.class_name(super_class).ok()
    };

    let iface_count = r.u16()? as usize;
    for _ in 0..iface_count {
        r.u16()?;
    }
    let field_count = r.u16()? as usize;
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        fields.push(parse_field_name(r, &cp)?);
    }
    let method_count = r.u16()? as usize;
    let mut methods = Vec::with_capacity(method_count);
    for _ in 0..method_count {
        methods.push(parse_method(r, &cp)?);
    }
    Ok(ClassFile {
        name,
        super_name,
        fields,
        methods,
        pool: cp,
    })
}

const ACC_STATIC: u16 = 0x0008;

fn parse_method(r: &mut Reader, cp: &ConstantPool) -> Result<Method> {
    let access = r.u16()?;
    let name = cp.utf8(r.u16()?)?;
    let descriptor = cp.utf8(r.u16()?)?;
    let attr_count = r.u16()? as usize;
    let mut code = None;
    for _ in 0..attr_count {
        let attr_name = cp.utf8(r.u16()?)?;
        let attr_len = r.u32()? as usize;
        let body = r.bytes(attr_len)?;
        if attr_name == "Code" {
            code = Some(extract_code(body)?);
        }
    }
    Ok(Method {
        name,
        descriptor,
        is_static: access & ACC_STATIC != 0,
        code,
    })
}

/// Code attribute layout: u2 max_stack, u2 max_locals, u4 code_length, u1 code[code_length], …
fn extract_code(body: &[u8]) -> Result<Vec<u8>> {
    let cr = &mut Reader::new(body);
    let _max_stack = cr.u16()?;
    let _max_locals = cr.u16()?;
    let code_len = cr.u32()? as usize;
    Ok(cr.bytes(code_len)?.to_vec())
}

/// Parse a field, returning its name and skipping the rest.
fn parse_field_name(r: &mut Reader, cp: &ConstantPool) -> Result<String> {
    r.u16()?; // access flags
    let name = cp.utf8(r.u16()?)?;
    r.u16()?; // descriptor index
    let attr_count = r.u16()? as usize;
    for _ in 0..attr_count {
        r.u16()?; // attr name index
        let attr_len = r.u32()? as usize;
        r.bytes(attr_len)?;
    }
    Ok(name)
}

// ── Constant pool ─────────────────────────────────────────────────────────────

#[derive(Clone)]
enum CpEntry {
    Utf8(String),
    Class(u16),            // name_index → Utf8
    Methodref(u16, u16),   // class_index, name_and_type_index
    NameAndType(u16, u16), // name_index, descriptor_index
    Other,
    Placeholder, // unused slot 0, and the slot following Long/Double
}

#[derive(Clone)]
struct ConstantPool {
    entries: Vec<CpEntry>,
}

impl ConstantPool {
    fn utf8(&self, idx: u16) -> Result<String> {
        match self.entries.get(idx as usize) {
            Some(CpEntry::Utf8(s)) => Ok(s.clone()),
            _ => Err(RavaError::Other(format!(
                "constant pool index {idx} is not a Utf8 entry"
            ))),
        }
    }

    fn class_name(&self, idx: u16) -> Result<String> {
        match self.entries.get(idx as usize) {
            Some(CpEntry::Class(name_idx)) => self.utf8(*name_idx),
            _ => Err(RavaError::Other(format!(
                "constant pool index {idx} is not a Class entry"
            ))),
        }
    }

    /// Resolve a `Methodref` index → (class name, method name, descriptor).
    fn method_ref(&self, idx: u16) -> Result<(String, String, String)> {
        let (class_idx, nat_idx) = match self.entries.get(idx as usize) {
            Some(CpEntry::Methodref(c, n)) => (*c, *n),
            _ => {
                return Err(RavaError::Other(format!(
                    "constant pool index {idx} is not a Methodref"
                )))
            }
        };
        let class = self.class_name(class_idx)?;
        let (name_idx, desc_idx) = match self.entries.get(nat_idx as usize) {
            Some(CpEntry::NameAndType(n, d)) => (*n, *d),
            _ => {
                return Err(RavaError::Other(format!(
                    "constant pool index {nat_idx} is not a NameAndType"
                )))
            }
        };
        Ok((class, self.utf8(name_idx)?, self.utf8(desc_idx)?))
    }
}

fn parse_constant_pool(r: &mut Reader) -> Result<ConstantPool> {
    let count = r.u16()? as usize; // = number of entries + 1
    let mut entries = vec![CpEntry::Placeholder; count];
    let mut i = 1;
    while i < count {
        let tag = r.u8()?;
        match tag {
            1 => {
                let len = r.u16()? as usize;
                let bytes = r.bytes(len)?;
                entries[i] = CpEntry::Utf8(String::from_utf8_lossy(bytes).into_owned());
            }
            7 => entries[i] = CpEntry::Class(r.u16()?), // Class
            8 | 16 | 19 | 20 => {
                r.u16()?; // String / MethodType / Module / Package
                entries[i] = CpEntry::Other;
            }
            3 | 4 => {
                r.u32()?; // Integer / Float
                entries[i] = CpEntry::Other;
            }
            9 | 10 | 11 => {
                // Fieldref / Methodref / InterfaceMethodref: class_index, name_and_type_index
                let class = r.u16()?;
                let nat = r.u16()?;
                entries[i] = CpEntry::Methodref(class, nat);
            }
            12 => {
                // NameAndType: name_index, descriptor_index
                let name = r.u16()?;
                let desc = r.u16()?;
                entries[i] = CpEntry::NameAndType(name, desc);
            }
            17 | 18 => {
                r.u16()?;
                r.u16()?; // (Invoke)Dynamic — not needed yet
                entries[i] = CpEntry::Other;
            }
            15 => {
                r.u8()?;
                r.u16()?; // MethodHandle
                entries[i] = CpEntry::Other;
            }
            5 | 6 => {
                r.u32()?;
                r.u32()?; // Long / Double occupy two pool slots
                entries[i] = CpEntry::Other;
                i += 1;
            }
            _ => {
                return Err(RavaError::Other(format!(
                    "unknown constant pool tag {tag} at entry {i}"
                )))
            }
        }
        i += 1;
    }
    Ok(ConstantPool { entries })
}

// ── Reader ────────────────────────────────────────────────────────────────────

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8> {
        let b = self
            .data
            .get(self.pos)
            .copied()
            .ok_or_else(|| RavaError::Other(format!("unexpected EOF at offset {}", self.pos)))?;
        self.pos += 1;
        Ok(b)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(((self.u8()? as u16) << 8) | self.u8()? as u16)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(((self.u16()? as u32) << 16) | self.u16()? as u32)
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos + n;
        if end > self.data.len() {
            return Err(RavaError::Other(format!(
                "unexpected EOF: need {n} bytes at offset {}",
                self.pos
            )));
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compiled from `class Add { static int add(int,int){...} int triple(int){...} }`.
    const ADD_CLASS: &[u8] = include_bytes!("fixtures/Add.class");

    #[test]
    fn parses_class_and_methods() {
        let cf = parse(ADD_CLASS).expect("parse Add.class");
        assert_eq!(cf.name, "Add");
        assert_eq!(cf.super_name.as_deref(), Some("java/lang/Object"));

        let add = cf.methods.iter().find(|m| m.name == "add").expect("add");
        assert_eq!(add.descriptor, "(II)I");
        assert!(add.is_static);
        // iload_0, iload_1, iadd, ireturn
        assert_eq!(add.code.as_deref(), Some(&[0x1a, 0x1b, 0x60, 0xac][..]));

        let triple = cf.methods.iter().find(|m| m.name == "triple").expect("triple");
        assert_eq!(triple.descriptor, "(I)I");
        assert!(!triple.is_static);

        assert!(cf.methods.iter().any(|m| m.name == "<init>"));
    }

    #[test]
    fn rejects_non_class_bytes() {
        assert!(parse(b"not a class file").is_err());
    }
}
