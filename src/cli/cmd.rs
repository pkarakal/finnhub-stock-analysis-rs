use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
pub struct CLIOptions {
    #[clap(short, long)]
    pub verbose: bool,
    #[clap(forbid_empty_values = true, required = true, short, long)]
    pub token: String,
    #[clap(forbid_empty_values = true, required = true, short, long)]
    pub stocks: Vec<String>,
}
