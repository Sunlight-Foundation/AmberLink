// amber-core/src/codegen/archive.rs
// Amberlink Archive (.ama) writer — the "JAR" analog for Amberlink v0.7.
//
// A .ama packages a compiled program (the .amc bytecode, stored under the
// reserved entry name "main") together with bundled data/resource files, so a
// program and its data ship as a single distributable archive. The VM reads
// the main entry and exposes the remaining entries as runtime resources.
//
// Binary layout (all integers little-endian):
//   "AMRA"            u8[4]     magic
//   version           u16       1
//   entry_count       u32
//   for each entry:
//     name_len        u32
//     name            u8[name_len]
//     data_len        u32
//     data            u8[data_len]

pub const MAIN_ENTRY: &str = "main";

/// Builds the .ama archive bytes from the main .amc bytes plus resource files.
/// Resources are given as (entry name, data) pairs.
pub fn build_archive(main_bytecode: Vec<u8>, resources: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    entries.push((MAIN_ENTRY.to_string(), main_bytecode));
    for (name, data) in resources {
        entries.push((name, data));
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"AMRA");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (name, data) in &entries {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}
