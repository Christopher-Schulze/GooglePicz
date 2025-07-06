#![warn(clippy::all)]
#![warn(rust_2018_idioms)]
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// Package format on Linux (deb, rpm or appimage)
    #[arg(long, value_parser = ["deb", "rpm", "appimage"])]
    format: Option<String>,
}

fn main() -> Result<(), packaging::PackagingError> {
    let args = Args::parse();
    if let Some(fmt) = args.format {
        std::env::set_var("LINUX_PACKAGE_FORMAT", fmt);
    }
    packaging::package_all()?;
    Ok(())
}
