use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Prints traces to local stdout instead of jaeger
    #[arg(short, long)]
    local: bool,
}

pub fn print_local() -> bool {
    let args = Args::parse();
    args.local
}
