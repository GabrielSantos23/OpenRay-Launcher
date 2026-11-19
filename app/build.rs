use std::fmt::Write;

fn main() {
    println!("cargo:rerun-if-changed=assets/app-icons/icon.ico");
    embed_resource::compile("openray.rc", embed_resource::NONE).manifest_optional().unwrap();
    println!("cargo:rerun-if-changed=assets/icons");
    let mut names: Vec<String> = std::fs::read_dir("assets/icons")
        .expect("assets/icons")
        .filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".svg").map(str::to_owned))
        .collect();
    names.sort();
    let mut code = String::from("pub fn svg_bytes(name: &str) -> Option<&'static [u8]> {\n    match name {\n");
    for name in names {
        writeln!(code, "        {name:?} => Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/icons/{name}.svg\"))),").unwrap();
    }
    code.push_str("        _ => None,\n    }\n}\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("icons.rs"), code).unwrap();
}
