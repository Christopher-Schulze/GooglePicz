fn main() -> Result<(), packaging::PackagingError> {
    packaging::package_all()?;
    Ok(())
}
