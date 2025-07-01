fn main() -> Result<(), Box<dyn std::error::Error>> {
    packaging::package_all()?;
    Ok(())
}
