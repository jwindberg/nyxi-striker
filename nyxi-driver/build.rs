fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use a bundled protoc so the build needs no system install; PROTOC still overrides.
    if std::env::var_os("PROTOC").is_none() {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }
    tonic_build::compile_protos("proto/driver.proto")?;
    Ok(())
}
