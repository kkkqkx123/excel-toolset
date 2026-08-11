mod cli;

fn main() {
    use clap::Parser;
    let cli = cli::Cli::parse();
    // The error JSON has already been printed by `execute`; here we only need to
    // surface failure through the exit code so CI and shell scripts can react.
    if cli::execute(&cli).is_err() {
        std::process::exit(1);
    }
}
