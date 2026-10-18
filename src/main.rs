use anyhow::Result;
use clap::Parser;
use rsopen::{launch_app_with_options, LaunchOptions};

#[derive(Parser, Debug)]
#[command(version, about = "A multiplatform app launcher", long_about = None)]
struct Args {
    /// Name of the application to launch
    #[arg(index = 1)]
    app_name: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Wait until the application exits
    #[arg(short, long)]
    blocking: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let options = LaunchOptions { verbose: args.verbose, blocking: args.blocking };
    if let Err(e) = launch_app_with_options(&args.app_name, options) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
