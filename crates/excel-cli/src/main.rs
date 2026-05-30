mod cli;

fn main() {
    use clap::Parser;
    let cli = cli::Cli::parse();
    cli::execute(&cli);
}
