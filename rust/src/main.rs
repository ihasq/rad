use clap::Parser;

#[derive(Parser)]
#[command(name = "rad", version = "0.0.1")]
#[command(about = "Rad source control management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {}

fn main() {
    let _cli = Cli::parse();
    // サブコマンドなしの場合は help を表示
    Cli::parse_from(["rad", "--help"]);
}
