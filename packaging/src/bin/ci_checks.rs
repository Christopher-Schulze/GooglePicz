fn main() -> Result<(), Box<dyn std::error::Error>> {
    packaging::utils::verify_metadata_package_name("googlepicz")?;
    packaging::utils::verify_artifact_names()?;
    println!("CI checks passed");
    Ok(())
}
