use anyhow::Result;
use clap::Parser;
use rsopen::launch_app;

#[derive(Parser, Debug)]
#[command(version, about = "A multiplatform app launcher", long_about = None)]
struct Args {
    /// Name of the application to launch
    #[arg(index = 1)]
    app_name: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let Err(e) = launch_app(&args.app_name, args.verbose) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
