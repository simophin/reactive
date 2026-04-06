mod encode;
mod pool;

use encode::*;
use pool::*;

use crate::class_def::{AccessFlags, ClassDef, MethodCode, MethodEntry};
use crate::output::{DexOutput, NativeMethod, NativeRegistrations};

// ───────────────────────────────────────────────── DEX constants ──────────
const DEX_MAGIC: &[u8; 8] = b"dex\n035\0";
const ENDIAN_CONSTANT: u32 = 0x12345678;
const HEADER_SIZE: u32 = 112;
const NO_INDEX: u32 = 0xFFFFFFFF;

// Map-list type codes
const TYPE_HEADER_ITEM: u16        = 0x0000;
const TYPE_STRING_ID_ITEM: u16     = 0x0001;
const TYPE_TYPE_ID_ITEM: u16       = 0x0002;
const TYPE_PROTO_ID_ITEM: u16      = 0x0003;
const TYPE_FIELD_ID_ITEM: u16      = 0x0004;
const TYPE_METHOD_ID_ITEM: u16     = 0x0005;
const TYPE_CLASS_DEF_ITEM: u16     = 0x0006;
const TYPE_TYPE_LIST: u16          = 0x1001;
const TYPE_STRING_DATA_ITEM: u16   = 0x2002;
const TYPE_CODE_ITEM: u16          = 0x2001;
const TYPE_CLASS_DATA_ITEM: u16    = 0x2000;
const TYPE_MAP_LIST: u16           = 0x1000;

// ─────────────────────────────────────────────────────────────────────────

/// All string literals that will appear in the DEX.
struct Strings<'a> {
    // Sorted after collection
    pool: SortedStrings,
    /// Back-reference to look up indices by content
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Strings<'a> {
    fn idx(&self, s: &str) -> u32 {
        self.pool.index_of(s)
    }
    fn len(&self) -> u32 {
        self.pool.len()
    }
}

// ─────────────────────────────────────────── collected ID tables ──────────

/// A resolved type, proto, field or method index.
#[allow(dead_code)] type TypeIdx   = u32;
#[allow(dead_code)] type ProtoIdx  = u32;
#[allow(dead_code)] type FieldIdx  = u32;
#[allow(dead_code)] type MethodIdx = u32;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TypeEntry {
    descriptor: String,
}

#[derive(Clone, PartialEq, Eq)]
struct ProtoEntry {
    shorty: String,
    return_type: String,
    params: Vec<String>,
}

impl PartialOrd for ProtoEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ProtoEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.return_type.cmp(&other.return_type)
            .then(self.params.cmp(&other.params))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FieldEntry {
    class: String,
    type_: String,
    name: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MethodIdEntry {
    class: String,
    name: String,
    proto: ProtoEntry,
}

// ─────────────────────────────────────── DexWriter ────────────────────────

pub struct DexWriter;

impl DexWriter {
    pub fn compile(def: ClassDef) -> Result<DexOutput, String> {
        Builder::new(def).build()
    }
}

struct Builder {
    def: ClassDef,
}

impl Builder {
    fn new(def: ClassDef) -> Self {
        Self { def }
    }

    fn build(self) -> Result<DexOutput, String> {
        // ── 1. Collect native registrations before consuming def ──────────
        let class_name = self.def.class_name.clone();
        let native_methods: Vec<NativeMethod> = self
            .def
            .methods
            .iter()
            .filter_map(|m| {
                if let MethodEntry::Native { name, descriptor, fn_ptr, .. } = m {
                    Some(NativeMethod {
                        name: name.clone(),
                        descriptor: descriptor.clone(),
                        fn_ptr: *fn_ptr,
                    })
                } else {
                    None
                }
            })
            .collect();

        // ── 2. Collect all string / type / proto / field / method IDs ─────
        let mut sp = StringPool::default();
        self.collect_strings(&mut sp);
        let strings = Strings {
            pool: sp.sorted(),
            _marker: std::marker::PhantomData,
        };

        let types   = self.collect_types(&strings);
        let protos  = self.collect_protos(&strings);
        let fields  = self.collect_fields(&strings);
        let methods = self.collect_methods(&strings, &protos);

        // ── 3. Serialise ──────────────────────────────────────────────────
        let bytes = self.serialise(&strings, &types, &protos, &fields, &methods)?;

        let registrations = NativeRegistrations {
            class_name,
            methods: native_methods,
        };
        Ok(DexOutput::new(bytes, registrations))
    }

    // ─────────────────────────── string collection ────────────────────────

    fn collect_strings(&self, sp: &mut StringPool) {
        let def = &self.def;

        // Class/super/interfaces as type descriptors
        sp.intern(class_desc(&def.class_name));
        sp.intern(class_desc(&def.superclass));
        for iface in &def.interfaces {
            sp.intern(class_desc(iface));
        }

        // Well-known strings always needed
        sp.intern("<init>");
        sp.intern("$$destroy");
        sp.intern("finalize");
        sp.intern("nativePtr");
        sp.intern("J");       // long type descriptor
        sp.intern("V");       // void
        sp.intern("Ljava/lang/Object;");

        // Fields
        for f in &def.fields {
            sp.intern(&f.name);
            sp.intern(&f.descriptor);
        }

        // Methods
        for m in &def.methods {
            match m {
                MethodEntry::Native { name, descriptor, .. }
                | MethodEntry::Coded { name, descriptor, .. } => {
                    sp.intern(name.as_str());
                    self.intern_descriptor(sp, descriptor);
                }
            }
            // Extra strings from code patterns
            if let MethodEntry::Coded { code, .. } = m {
                match code {
                    MethodCode::Constructor { superclass, super_descriptor }
                    | MethodCode::ByoConstructor { superclass, super_descriptor } => {
                        sp.intern(class_desc(superclass));
                        self.intern_descriptor(sp, super_descriptor);
                        if matches!(code, MethodCode::ByoConstructor { .. }) {
                            let byo = byo_descriptor(super_descriptor);
                            self.intern_descriptor(sp, &byo);
                        }
                    }
                    MethodCode::SuperAccessor { superclass, method_name, descriptor } => {
                        sp.intern(class_desc(superclass));
                        sp.intern(method_name.as_str());
                        self.intern_descriptor(sp, descriptor);
                    }
                    MethodCode::Finalize => {
                        sp.intern("Ljava/lang/Object;");
                    }
                }
            }
        }
        // Shorty descriptors are also strings
        for m in &def.methods {
            let desc = match m {
                MethodEntry::Native { descriptor, .. } => descriptor.as_str(),
                MethodEntry::Coded { descriptor, .. } => descriptor.as_str(),
            };
            sp.intern(shorty(desc));
        }
    }

    fn intern_descriptor(&self, sp: &mut StringPool, descriptor: &str) {
        sp.intern(descriptor);
        // Also intern component types
        for p in parse_param_descriptors(descriptor) {
            sp.intern(&p);
        }
        sp.intern(return_descriptor(descriptor));
        sp.intern(shorty(descriptor));
    }

    // ─────────────────────────── type table ───────────────────────────────

    fn collect_types(&self, strings: &Strings<'_>) -> Vec<TypeEntry> {
        let mut types: Vec<TypeEntry> = strings
            .pool
            .0
            .iter()
            .filter(|s| is_type_descriptor(s))
            .map(|s| TypeEntry { descriptor: s.clone() })
            .collect();
        types.sort();
        types
    }

    fn type_idx(types: &[TypeEntry], desc: &str) -> TypeIdx {
        types
            .binary_search_by(|e| e.descriptor.as_str().cmp(desc))
            .unwrap_or_else(|_| panic!("type not collected: {desc:?}")) as u32
    }

    // ─────────────────────────── proto table ──────────────────────────────

    fn collect_protos(&self, strings: &Strings<'_>) -> Vec<ProtoEntry> {
        let _ = strings;
        let mut seen: Vec<ProtoEntry> = Vec::new();
        let descriptors: Vec<&str> = self.def.methods.iter().map(|m| match m {
            MethodEntry::Native { descriptor, .. } => descriptor.as_str(),
            MethodEntry::Coded { descriptor, .. } => descriptor.as_str(),
        }).collect();

        let mut all_descs: Vec<String> = descriptors.iter().map(|s| s.to_string()).collect();
        // Also BYO constructors have a synthetic descriptor
        for m in &self.def.methods {
            if let MethodEntry::Coded { code: MethodCode::ByoConstructor { super_descriptor, .. }, .. } = m {
                all_descs.push(byo_descriptor(super_descriptor));
            }
        }
        // finalize / $$destroy share ()V — already in method descs

        for desc in &all_descs {
            let entry = ProtoEntry {
                shorty: shorty(desc),
                return_type: return_descriptor(desc).to_string(),
                params: parse_param_descriptors(desc),
            };
            if !seen.contains(&entry) {
                seen.push(entry);
            }
        }
        seen.sort();
        seen
    }

    #[allow(dead_code)]
    fn proto_idx(protos: &[ProtoEntry], descriptor: &str) -> ProtoIdx {
        let target = ProtoEntry {
            shorty: shorty(descriptor),
            return_type: return_descriptor(descriptor).to_string(),
            params: parse_param_descriptors(descriptor),
        };
        protos
            .iter()
            .position(|p| p == &target)
            .unwrap_or_else(|| panic!("proto not collected: {descriptor:?}")) as u32
    }

    // ─────────────────────────── field table ──────────────────────────────

    fn collect_fields(&self, _strings: &Strings<'_>) -> Vec<FieldEntry> {
        let mut fields: Vec<FieldEntry> = self
            .def
            .fields
            .iter()
            .map(|f| FieldEntry {
                class: class_desc(&self.def.class_name),
                type_: f.descriptor.clone(),
                name: f.name.clone(),
            })
            .collect();
        fields.sort();
        fields
    }

    fn field_idx(fields: &[FieldEntry], class_desc: &str, name: &str) -> FieldIdx {
        fields
            .iter()
            .position(|f| f.class == class_desc && f.name == name)
            .unwrap_or_else(|| panic!("field not collected: {class_desc}::{name}")) as u32
    }

    // ─────────────────────────── method table ─────────────────────────────

    fn collect_methods(
        &self,
        _strings: &Strings<'_>,
        _protos: &[ProtoEntry],
    ) -> Vec<MethodIdEntry> {
        let class = class_desc(&self.def.class_name);
        let super_ = class_desc(&self.def.superclass);

        let mut methods: Vec<MethodIdEntry> = Vec::new();

        let add = |methods: &mut Vec<MethodIdEntry>, class: &str, name: &str, desc: &str| {
            let entry = MethodIdEntry {
                class: class.to_string(),
                name: name.to_string(),
                proto: ProtoEntry {
                    shorty: shorty(desc),
                    return_type: return_descriptor(desc).to_string(),
                    params: parse_param_descriptors(desc),
                },
            };
            if !methods.contains(&entry) {
                methods.push(entry);
            }
        };

        // $$destroy and finalize are always present
        add(&mut methods, &class, "$$destroy", "()V");
        add(&mut methods, &class, "finalize", "()V");
        add(&mut methods, "Ljava/lang/Object;", "finalize", "()V");

        for m in &self.def.methods {
            let (name, desc) = match m {
                MethodEntry::Native { name, descriptor, .. } => (name.as_str(), descriptor.as_str()),
                MethodEntry::Coded { name, descriptor, .. } => (name.as_str(), descriptor.as_str()),
            };
            add(&mut methods, &class, name, desc);

            if let MethodEntry::Coded { code, .. } = m {
                match code {
                    MethodCode::Constructor { super_descriptor, .. }
                    | MethodCode::ByoConstructor { super_descriptor, .. } => {
                        add(&mut methods, &super_, "<init>", super_descriptor);
                        if let MethodCode::ByoConstructor { super_descriptor, .. } = code {
                            let byo = byo_descriptor(super_descriptor);
                            add(&mut methods, &class, "<init>", &byo);
                        }
                    }
                    MethodCode::SuperAccessor { superclass, method_name, descriptor } => {
                        add(&mut methods, &class_desc(superclass), method_name, descriptor);
                    }
                    MethodCode::Finalize => {}
                }
            }
        }

        methods.sort();
        methods
    }

    fn method_idx(methods: &[MethodIdEntry], class: &str, name: &str, desc: &str) -> MethodIdx {
        let target = MethodIdEntry {
            class: class.to_string(),
            name: name.to_string(),
            proto: ProtoEntry {
                shorty: shorty(desc),
                return_type: return_descriptor(desc).to_string(),
                params: parse_param_descriptors(desc),
            },
        };
        methods
            .iter()
            .position(|m| m == &target)
            .unwrap_or_else(|| panic!("method not collected: {class}::{name}{desc}")) as u32
    }

    // ─────────────────────────────────────── serialise ────────────────────

    fn serialise(
        &self,
        strings: &Strings<'_>,
        types: &[TypeEntry],
        protos: &[ProtoEntry],
        fields: &[FieldEntry],
        methods: &[MethodIdEntry],
    ) -> Result<Vec<u8>, String> {
        let mut buf: Vec<u8> = Vec::new();

        // ── Header placeholder (112 bytes) ────────────────────────────────
        buf.extend_from_slice(DEX_MAGIC);
        buf.extend_from_slice(&[0u8; 4]); // checksum placeholder
        buf.extend_from_slice(&[0u8; 20]); // SHA-1 placeholder
        // file_size placeholder
        write_u32_le(&mut buf, 0);
        write_u32_le(&mut buf, HEADER_SIZE);
        write_u32_le(&mut buf, ENDIAN_CONSTANT);
        // link_size, link_off
        write_u32_le(&mut buf, 0);
        write_u32_le(&mut buf, 0);
        // map_off placeholder
        write_u32_le(&mut buf, 0);
        // string_ids_size, string_ids_off
        write_u32_le(&mut buf, strings.len());
        write_u32_le(&mut buf, HEADER_SIZE);
        // type_ids_size, type_ids_off
        let type_ids_off = HEADER_SIZE + strings.len() * 4;
        write_u32_le(&mut buf, types.len() as u32);
        write_u32_le(&mut buf, type_ids_off);
        // proto_ids_size, proto_ids_off
        let proto_ids_off = type_ids_off + types.len() as u32 * 4;
        write_u32_le(&mut buf, protos.len() as u32);
        write_u32_le(&mut buf, proto_ids_off);
        // field_ids_size, field_ids_off
        let field_ids_off = proto_ids_off + protos.len() as u32 * 12;
        write_u32_le(&mut buf, fields.len() as u32);
        write_u32_le(&mut buf, field_ids_off);
        // method_ids_size, method_ids_off
        let method_ids_off = field_ids_off + fields.len() as u32 * 8;
        write_u32_le(&mut buf, methods.len() as u32);
        write_u32_le(&mut buf, method_ids_off);
        // class_defs_size, class_defs_off
        let class_defs_off = method_ids_off + methods.len() as u32 * 8;
        write_u32_le(&mut buf, 1u32);
        write_u32_le(&mut buf, class_defs_off);
        // data_size, data_off placeholders
        write_u32_le(&mut buf, 0);
        write_u32_le(&mut buf, 0);

        assert_eq!(buf.len(), 112, "header must be 112 bytes");

        // ── String ID section ─────────────────────────────────────────────
        // Offsets into the data section; filled in once we know the data layout
        let string_ids_start = buf.len();
        for _ in 0..strings.len() {
            write_u32_le(&mut buf, 0); // placeholder
        }

        // ── Type ID section ───────────────────────────────────────────────
        for t in types {
            write_u32_le(&mut buf, strings.idx(&t.descriptor));
        }

        // ── Proto ID section ──────────────────────────────────────────────
        // Each entry: shorty_idx(u32), return_type_idx(u32), parameters_off(u32)
        // parameters_off values need to be backfilled after we write type_lists
        let proto_ids_start = buf.len();
        for p in protos {
            write_u32_le(&mut buf, strings.idx(&p.shorty));
            write_u32_le(&mut buf, Self::type_idx(types, &p.return_type));
            write_u32_le(&mut buf, 0); // parameters_off placeholder
        }

        // ── Field ID section ──────────────────────────────────────────────
        for f in fields {
            let class_idx = Self::type_idx(types, &f.class) as u16;
            let type_idx  = Self::type_idx(types, &f.type_) as u16;
            let name_idx  = strings.idx(&f.name);
            write_u16_le(&mut buf, class_idx);
            write_u16_le(&mut buf, type_idx);
            write_u32_le(&mut buf, name_idx);
        }

        // ── Method ID section ─────────────────────────────────────────────
        for m in methods {
            let class_idx = Self::type_idx(types, &m.class) as u16;
            let proto_idx = protos.iter().position(|p| {
                p.shorty == m.proto.shorty
                    && p.return_type == m.proto.return_type
                    && p.params == m.proto.params
            }).unwrap() as u16;
            let name_idx = strings.idx(&m.name);
            write_u16_le(&mut buf, class_idx);
            write_u16_le(&mut buf, proto_idx);
            write_u32_le(&mut buf, name_idx);
        }

        // ── Class def section (1 entry, 32 bytes) ─────────────────────────
        let class_def_start = buf.len();
        assert_eq!(class_def_start as u32, class_defs_off);
        let class_type_idx  = Self::type_idx(types, &class_desc(&self.def.class_name));
        let super_type_idx  = Self::type_idx(types, &class_desc(&self.def.superclass));
        write_u32_le(&mut buf, class_type_idx);
        write_u32_le(&mut buf, AccessFlags::PUBLIC.0);
        write_u32_le(&mut buf, super_type_idx);
        // interfaces_off placeholder
        let interfaces_off_pos = buf.len();
        write_u32_le(&mut buf, 0);
        write_u32_le(&mut buf, NO_INDEX); // source_file_idx
        write_u32_le(&mut buf, 0);        // annotations_off
        // class_data_off placeholder
        let class_data_off_pos = buf.len();
        write_u32_le(&mut buf, 0);
        write_u32_le(&mut buf, 0); // static_values_off

        // ════════════════════════════ DATA SECTION ════════════════════════
        let data_start = buf.len() as u32;

        // ── String data items ─────────────────────────────────────────────
        let mut string_offsets: Vec<u32> = Vec::new();
        for s in &strings.pool.0 {
            string_offsets.push(buf.len() as u32);
            let encoded = mutf8_encode(s);
            write_uleb128(&mut buf, encoded.len() as u32);
            buf.extend_from_slice(&encoded);
            buf.push(0); // null terminator
        }
        // Backfill string ID offsets
        for (i, &off) in string_offsets.iter().enumerate() {
            let pos = string_ids_start + i * 4;
            buf[pos..pos + 4].copy_from_slice(&off.to_le_bytes());
        }

        // ── Interfaces type_list ──────────────────────────────────────────
        if !self.def.interfaces.is_empty() {
            align_to(&mut buf, 4);
            let iface_list_off = buf.len() as u32;
            // Backfill interfaces_off in class def
            buf[interfaces_off_pos..interfaces_off_pos + 4]
                .copy_from_slice(&iface_list_off.to_le_bytes());
            write_u32_le(&mut buf, self.def.interfaces.len() as u32);
            for iface in &self.def.interfaces {
                let idx = Self::type_idx(types, &class_desc(iface)) as u16;
                write_u16_le(&mut buf, idx);
            }
            // Pad to 4-byte boundary
            align_to(&mut buf, 4);
        }

        // ── Proto parameter type_lists ────────────────────────────────────
        let mut proto_param_offsets: Vec<u32> = Vec::new();
        for p in protos {
            if p.params.is_empty() {
                proto_param_offsets.push(0);
            } else {
                align_to(&mut buf, 4);
                proto_param_offsets.push(buf.len() as u32);
                write_u32_le(&mut buf, p.params.len() as u32);
                for param in &p.params {
                    let idx = Self::type_idx(types, param) as u16;
                    write_u16_le(&mut buf, idx);
                }
                align_to(&mut buf, 4);
            }
        }
        // Backfill proto parameters_off
        for (i, &off) in proto_param_offsets.iter().enumerate() {
            let pos = proto_ids_start + i * 12 + 8; // 8 = shorty(4) + return(4)
            buf[pos..pos + 4].copy_from_slice(&off.to_le_bytes());
        }

        // ── Code items ───────────────────────────────────────────────────
        let my_class = class_desc(&self.def.class_name);
        let my_super = class_desc(&self.def.superclass);

        struct CodeRef {
            method_class: String,
            method_name: String,
            method_desc: String,
            off: u32,
        }
        let mut code_offsets: Vec<CodeRef> = Vec::new();

        // We need a $$destroy code item and a finalize code item in addition to
        // user-provided Coded methods.
        struct CodeGen {
            method_name: String,
            method_desc: String,
            registers: u16,
            ins: u16,
            outs: u16,
            insns: Vec<u16>,
        }

        let mut code_items: Vec<CodeGen> = Vec::new();

        for m in &self.def.methods {
            if let MethodEntry::Coded { name, descriptor, code, .. } = m {
                let insns = self.emit_bytecode(
                    code,
                    descriptor,
                    methods,
                    fields,
                    &my_class,
                    &my_super,
                );
                let (regs, ins, outs) = self.register_counts(code, descriptor);
                code_items.push(CodeGen {
                    method_name: name.clone(),
                    method_desc: descriptor.clone(),
                    registers: regs,
                    ins,
                    outs,
                    insns,
                });
            }
        }

        for cg in &code_items {
            align_to(&mut buf, 4);
            let off = buf.len() as u32;
            code_offsets.push(CodeRef {
                method_class: my_class.clone(),
                method_name: cg.method_name.clone(),
                method_desc: cg.method_desc.clone(),
                off,
            });
            write_u16_le(&mut buf, cg.registers);
            write_u16_le(&mut buf, cg.ins);
            write_u16_le(&mut buf, cg.outs);
            write_u16_le(&mut buf, 0); // tries_size
            write_u32_le(&mut buf, 0); // debug_info_off
            write_u32_le(&mut buf, cg.insns.len() as u32);
            for &insn in &cg.insns {
                write_u16_le(&mut buf, insn);
            }
        }

        // ── Class data item ───────────────────────────────────────────────
        align_to(&mut buf, 4);
        let class_data_off = buf.len() as u32;
        buf[class_data_off_pos..class_data_off_pos + 4]
            .copy_from_slice(&class_data_off.to_le_bytes());

        // Partition methods into direct (constructor, private, static) and virtual
        let (direct_entries, virtual_entries) = self.partition_methods(methods, &my_class);

        // instance fields
        write_uleb128(&mut buf, 0); // static_fields_size
        write_uleb128(&mut buf, self.def.fields.len() as u32);
        write_uleb128(&mut buf, direct_entries.len() as u32);
        write_uleb128(&mut buf, virtual_entries.len() as u32);

        // instance field entries (sorted by field_idx diff)
        let mut prev_fidx = 0u32;
        for f in &self.def.fields {
            let fidx = Self::field_idx(fields, &my_class, &f.name);
            write_uleb128(&mut buf, fidx - prev_fidx);
            prev_fidx = fidx;
            write_uleb128(&mut buf, f.access.0);
        }

        // Helper: look up code offset for a method (0 = native / no code)
        let code_off_for = |class: &str, name: &str, desc: &str| -> u32 {
            code_offsets
                .iter()
                .find(|c| c.method_class == class && c.method_name == name && c.method_desc == desc)
                .map(|c| c.off)
                .unwrap_or(0)
        };

        // direct methods
        let mut prev_midx = 0u32;
        for (midx, name, desc, flags) in &direct_entries {
            write_uleb128(&mut buf, midx - prev_midx);
            prev_midx = *midx;
            write_uleb128(&mut buf, flags.0);
            write_uleb128(&mut buf, code_off_for(&my_class, name, desc));
        }

        // virtual methods
        let mut prev_midx = 0u32;
        for (midx, name, desc, flags) in &virtual_entries {
            write_uleb128(&mut buf, midx - prev_midx);
            prev_midx = *midx;
            write_uleb128(&mut buf, flags.0);
            write_uleb128(&mut buf, code_off_for(&my_class, name, desc));
        }

        // ── Map list ──────────────────────────────────────────────────────
        align_to(&mut buf, 4);
        let map_off = buf.len() as u32;

        let map_entries: Vec<(u16, u32, u32)> = vec![
            (TYPE_HEADER_ITEM,      1,                      0),
            (TYPE_STRING_ID_ITEM,   strings.len(),          HEADER_SIZE),
            (TYPE_TYPE_ID_ITEM,     types.len() as u32,     type_ids_off),
            (TYPE_PROTO_ID_ITEM,    protos.len() as u32,    proto_ids_off),
            (TYPE_FIELD_ID_ITEM,    fields.len() as u32,    field_ids_off),
            (TYPE_METHOD_ID_ITEM,   methods.len() as u32,   method_ids_off),
            (TYPE_CLASS_DEF_ITEM,   1,                      class_defs_off),
            (TYPE_STRING_DATA_ITEM, strings.len(),          *string_offsets.first().unwrap_or(&data_start)),
            (TYPE_TYPE_LIST,        (proto_param_offsets.iter().filter(|&&x| x != 0).count() + if self.def.interfaces.is_empty() { 0 } else { 1 }) as u32, 0),
            (TYPE_CODE_ITEM,        code_items.len() as u32, code_offsets.first().map(|c| c.off).unwrap_or(0)),
            (TYPE_CLASS_DATA_ITEM,  1,                      class_data_off),
            (TYPE_MAP_LIST,         1,                      map_off),
        ];

        // Filter out zero-count entries
        let map_entries: Vec<_> = map_entries.into_iter().filter(|(_, cnt, _)| *cnt > 0).collect();

        write_u32_le(&mut buf, map_entries.len() as u32);
        for (type_code, count, off) in &map_entries {
            write_u16_le(&mut buf, *type_code);
            write_u16_le(&mut buf, 0); // unused
            write_u32_le(&mut buf, *count);
            write_u32_le(&mut buf, *off);
        }

        // ── Backfill header fields ─────────────────────────────────────────
        let file_size = buf.len() as u32;
        let data_size = file_size - data_start;

        // file_size @ offset 32
        buf[32..36].copy_from_slice(&file_size.to_le_bytes());
        // map_off @ offset 52
        buf[52..56].copy_from_slice(&map_off.to_le_bytes());
        // data_size @ offset 104
        buf[104..108].copy_from_slice(&data_size.to_le_bytes());
        // data_off @ offset 108
        buf[108..112].copy_from_slice(&data_start.to_le_bytes());

        // SHA-1 signature (covers everything after the 32-byte checksum field)
        let sig = sha1(&buf[32..]);
        buf[12..32].copy_from_slice(&sig);

        // Adler-32 checksum (covers everything after the 8-byte checksum field)
        let csum = adler32(&buf[12..]);
        buf[8..12].copy_from_slice(&csum.to_le_bytes());

        Ok(buf)
    }

    // ──────────────────────────── bytecode emission ───────────────────────

    fn register_counts(&self, code: &MethodCode, descriptor: &str) -> (u16, u16, u16) {
        // (registers_size, ins_size, outs_size)
        let params = parse_param_descriptors(descriptor);
        // Count wide (long/double) params — each takes 2 register slots
        let param_slots: u16 = params.iter().map(|p| {
            if p == "J" || p == "D" { 2 } else { 1 }
        }).sum::<u16>() + 1; // +1 for `this`

        match code {
            MethodCode::Constructor { super_descriptor, .. } => {
                let sp = parse_param_descriptors(super_descriptor);
                let out_slots: u16 = sp.iter().map(|p| {
                    if p == "J" || p == "D" { 2 } else { 1 }
                }).sum::<u16>() + 1;
                (param_slots, param_slots, out_slots.max(param_slots))
            }
            MethodCode::ByoConstructor { super_descriptor, .. } => {
                // extra long param
                let sp = parse_param_descriptors(super_descriptor);
                let out_slots: u16 = sp.iter().map(|p| {
                    if p == "J" || p == "D" { 2 } else { 1 }
                }).sum::<u16>() + 1;
                // ins includes the extra long (2 regs)
                let ins = param_slots + 2;
                (ins, ins, out_slots.max(ins))
            }
            MethodCode::SuperAccessor { descriptor, .. } => {
                let sp = parse_param_descriptors(descriptor);
                let out_slots: u16 = sp.iter().map(|p| {
                    if p == "J" || p == "D" { 2 } else { 1 }
                }).sum::<u16>() + 1;
                (out_slots, out_slots, out_slots)
            }
            MethodCode::Finalize => (1, 1, 1),
        }
    }

    fn emit_bytecode(
        &self,
        code: &MethodCode,
        _descriptor: &str,
        methods: &[MethodIdEntry],
        fields: &[FieldEntry],
        my_class: &str,
        _my_super: &str,
    ) -> Vec<u16> {
        match code {
            MethodCode::Constructor { superclass, super_descriptor } => {
                // invoke-direct {p0..pN}, super.<init>(args)
                // invoke-direct {p0..pN}, this.$$init(args)
                // return-void
                let super_midx = Self::method_idx(methods, &class_desc(superclass), "<init>", super_descriptor);
                let init_midx  = Self::method_idx(methods, my_class, "$$init", super_descriptor);
                let params = parse_param_descriptors(super_descriptor);
                let arg_regs: Vec<u8> = self.param_regs(&params, 1);
                let mut insns = Vec::new();
                emit_invoke(&mut insns, 0x70, super_midx, &arg_regs);
                emit_invoke(&mut insns, 0x70, init_midx, &arg_regs);
                insns.push(0x000e); // return-void
                insns
            }
            MethodCode::ByoConstructor { superclass, super_descriptor } => {
                // invoke-direct {p0..pN}, super.<init>(args)
                // iput-wide p_last, p0, nativePtr
                // return-void
                let super_midx = Self::method_idx(methods, &class_desc(superclass), "<init>", super_descriptor);
                let params = parse_param_descriptors(super_descriptor);
                let arg_regs: Vec<u8> = self.param_regs(&params, 1);
                // native ptr long starts right after the normal args
                let nptr_reg = 1u8 + arg_regs.last().copied().unwrap_or(0) + 1;
                let fidx = Self::field_idx(fields, my_class, "nativePtr");
                let mut insns = Vec::new();
                emit_invoke(&mut insns, 0x70, super_midx, &arg_regs);
                // iput-wide vA, vB, field — format 22c: [B<<4|A, 0x59] [field@]
                insns.push(((0u8 as u16) << 12) | ((nptr_reg as u16) << 8) | 0x59);
                insns.push(fidx as u16);
                insns.push(0x000e);
                insns
            }
            MethodCode::SuperAccessor { superclass, method_name, descriptor } => {
                // invoke-super {p0..pN}, super.method(args)
                // return-void  (or return appropriate type)
                let super_midx = Self::method_idx(methods, &class_desc(superclass), method_name, descriptor);
                let params = parse_param_descriptors(descriptor);
                let arg_regs: Vec<u8> = self.param_regs(&params, 1);
                let mut insns = Vec::new();
                emit_invoke(&mut insns, 0x6f, super_midx, &arg_regs);
                let ret = return_descriptor(descriptor);
                insns.push(return_opcode(ret));
                insns
            }
            MethodCode::Finalize => {
                // invoke-direct {p0}, this.$$destroy()V
                // invoke-super  {p0}, Object.finalize()V
                // return-void
                let destroy_midx = Self::method_idx(methods, my_class, "$$destroy", "()V");
                let finalize_midx = Self::method_idx(methods, "Ljava/lang/Object;", "finalize", "()V");
                let mut insns = Vec::new();
                emit_invoke(&mut insns, 0x70, destroy_midx, &[0]);
                emit_invoke(&mut insns, 0x6f, finalize_midx, &[0]);
                insns.push(0x000e);
                insns
            }
        }
    }

    /// Build the ordered register list [p0=this, p1..] for an invoke.
    fn param_regs(&self, params: &[String], first_reg: u8) -> Vec<u8> {
        let mut regs = vec![0u8]; // p0 = this
        let mut next = first_reg;
        for p in params {
            regs.push(next);
            next += if p == "J" || p == "D" { 2 } else { 1 };
        }
        regs
    }

    // ──────────────────────── direct / virtual partition ──────────────────

    fn partition_methods(
        &self,
        methods: &[MethodIdEntry],
        my_class: &str,
    ) -> (Vec<(u32, String, String, AccessFlags)>, Vec<(u32, String, String, AccessFlags)>) {
        let mut direct: Vec<(u32, String, String, AccessFlags)> = Vec::new();
        let mut virt:   Vec<(u32, String, String, AccessFlags)> = Vec::new();

        let lookup_flags = |name: &str, desc: &str| -> AccessFlags {
            for m in &self.def.methods {
                match m {
                    MethodEntry::Native { name: n, descriptor: d, access, .. }
                    | MethodEntry::Coded { name: n, descriptor: d, access, .. }
                        if n == name && d == desc => return *access,
                    _ => {}
                }
            }
            // builtins
            if name == "$$destroy" {
                return AccessFlags::PRIVATE;
            }
            if name == "finalize" {
                return AccessFlags::PROTECTED;
            }
            AccessFlags::PUBLIC
        };

        for (idx, m) in methods.iter().enumerate() {
            if m.class != my_class { continue; }
            let flags = lookup_flags(&m.name, &self.proto_to_desc(&m.proto));
            let is_direct = m.name == "<init>"
                || m.name == "<clinit>"
                || flags.contains(AccessFlags::PRIVATE);
            let midx = idx as u32;
            let desc = self.proto_to_desc(&m.proto);
            if is_direct {
                direct.push((midx, m.name.clone(), desc, flags));
            } else {
                virt.push((midx, m.name.clone(), desc, flags));
            }
        }

        direct.sort_by_key(|(i, _, _, _)| *i);
        virt.sort_by_key(|(i, _, _, _)| *i);
        (direct, virt)
    }

    fn proto_to_desc(&self, proto: &ProtoEntry) -> String {
        let params: String = proto.params.join("");
        format!("({params}){}", proto.return_type)
    }
}

// ──────────────────────────── helpers ─────────────────────────────────────

/// Convert a class name like "android/view/View" to a type descriptor "Landroid/view/View;".
pub fn class_desc(name: &str) -> String {
    if name.starts_with('[') || (name.len() == 1 && "VZBSCIJFD".contains(name)) {
        name.to_string()
    } else if name.starts_with('L') && name.ends_with(';') {
        name.to_string()
    } else {
        format!("L{name};")
    }
}

/// True if the string looks like a valid JNI type descriptor.
fn is_type_descriptor(s: &str) -> bool {
    if s.is_empty() { return false; }
    match s.chars().next().unwrap() {
        'V' | 'Z' | 'B' | 'S' | 'C' | 'I' | 'J' | 'F' | 'D' => s.len() == 1,
        'L' => s.ends_with(';'),
        '[' => true,
        _ => false,
    }
}

/// Encode a Rust string as MUTF-8 bytes (for DEX string data items).
/// For ASCII strings (the common case) this is identical to UTF-8.
fn mutf8_encode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        if c == '\0' {
            out.extend_from_slice(&[0xc0, 0x80]);
        } else {
            let mut tmp = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
        }
    }
    out
}

/// Emit a `35c`-format invoke instruction (up to 5 registers).
fn emit_invoke(insns: &mut Vec<u16>, opcode: u8, method_idx: u32, regs: &[u8]) {
    let count = regs.len().min(5) as u8;
    // Word 1: [count<<4 | reg_g, opcode]  (reg_g used only when count==5)
    let reg_g = if count == 5 { regs[4] } else { 0 };
    insns.push(((count as u16) << 12) | ((reg_g as u16) << 8) | opcode as u16);
    // Word 2: method index (low 16 bits; we panic if > 0xFFFF)
    assert!(method_idx <= 0xFFFF, "method index overflow");
    insns.push(method_idx as u16);
    // Word 3: packed regs DCBA (nibbles)
    let reg = |i| if i < regs.len() { regs[i] as u16 } else { 0 };
    insns.push((reg(3) << 12) | (reg(2) << 8) | (reg(1) << 4) | reg(0));
}

fn return_opcode(ret_desc: &str) -> u16 {
    match ret_desc.chars().next().unwrap_or('V') {
        'V'             => 0x000e, // return-void
        'L' | '['       => 0x0011, // return-object
        'J' | 'D'       => 0x0010, // return-wide
        _               => 0x000f, // return (primitive)
    }
}
