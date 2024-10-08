use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(false)
        // derive serialize to support json
        .type_attribute(".", "#[derive(serde::Serialize)]")
        .type_attribute(".", "#[serde(rename_all = \"PascalCase\")]")
        .compile(&["proto/runtime.proto"], &["proto"])?;
    Ok(())
}
