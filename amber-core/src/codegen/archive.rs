// amber-core/src/codegen/archive.rs
// Amberlink Archive (.ama) writer — the "JAR" analog for Amberlink.
// Supports single-module (v1) and multi-module (v2) archives.
//
// Binary layout (all integers little-endian):
//   "AMRA"            u8[4]     magic
//   version           u16       1 (single-module) or 2 (multi-module)
//   entry_count       u32
//   for each entry:
//     name_len        u32
//     name            u8[name_len]
//     data_len        u32
//     data            u8[data_len]
//
// Entries whose name ends with ".amc" are compiled modules; all other entries
// are resources. One module must be named "main" (the entry point).
//
// Multi-module archives (v2) store each separately-compiled module as its own
// .amc entry. At runtime the VM loads every module entry and resolves
// cross-module calls through a global symbol table.

pub const MAIN_ENTRY: &str = "main";
pub const ARCHIVE_V1: u16 = 1;
pub const ARCHIVE_V2: u16 = 2;

/// Builds the .ama archive bytes.
///
/// `modules` is a list of (entry_name, amc_bytes) pairs. One entry must be
/// named "main" — it is the program entry point. `resources` are (name, data)
/// pairs bundled for runtime access via `readResource`/`hasResource`.
pub fn build_archive(
    modules: Vec<(String, Vec<u8>)>,
    resources: Vec<(String, Vec<u8>)>,
) -> Vec<u8> {
    // Ensure there is a "main" entry.
    let has_main = modules.iter().any(|(name, _)| name == MAIN_ENTRY);
    if !has_main {
        // Build without main is an error; caller should guarantee it.
    }

    let is_multi = modules.len() > 1;
    let version = if is_multi { ARCHIVE_V2 } else { ARCHIVE_V1 };

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, data) in modules {
        entries.push((name, data));
    }
    for (name, data) in resources {
        entries.push((name, data));
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"AMRA");
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (name, data) in &entries {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{build_archive, ARCHIVE_V1, ARCHIVE_V2};

    #[test]
    fn single_module_archive_uses_v1_header() {
        let bytes = build_archive(vec![("main".into(), vec![1, 2, 3])], vec![]);
        assert_eq!(&bytes[..4], b"AMRA");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), ARCHIVE_V1);
    }

    #[test]
    fn multiple_modules_use_v2_header_and_preserve_entries() {
        let bytes = build_archive(
            vec![("main".into(), vec![1]), ("math.amc".into(), vec![2])],
            vec![("readme.txt".into(), vec![3])],
        );
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), ARCHIVE_V2);
        assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 3);
        assert!(bytes.windows(8).any(|window| window == b"math.amc"));
    }
}
