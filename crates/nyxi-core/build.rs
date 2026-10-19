// Embed the screen animation library (animations/*.txt) into the crate.
use std::{env, fs, path::Path};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../animations");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = fs::read_dir(&dir)
        .expect("animations/ directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    let mut out = String::from("pub const LIBRARY: &[(&str, &str)] = &[\n");
    for path in files {
        let slug = path.file_stem().unwrap().to_string_lossy();
        let full = fs::canonicalize(&path).unwrap();
        out += &format!("    ({slug:?}, include_str!({:?})),\n", full.display().to_string());
    }
    out += "];\n";
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("library.rs"), out).unwrap();
}
