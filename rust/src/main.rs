use clap::Parser;
use std::io::Read;

mod types;
mod crypto;
mod sign;
mod verify;

#[derive(Parser)]
#[command(name = "rad", version = "0.0.1")]
#[command(about = "Rad source control management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Generate Ed25519 key pair
    Keygen,
    /// Sign an operation from stdin
    Sign {
        #[arg(long)]
        secret_key: String,
    },
    /// Verify a signed operation from stdin
    Verify {
        #[arg(long)]
        public_key: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Keygen) => {
            let kp = crypto::generate_keypair();
            println!("{}", crypto::format_keypair(&kp));
        }
        Some(Commands::Sign { secret_key }) => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            let sig = sign::sign_operation(input.trim(), &secret_key);
            let output = sign::inject_signature(input.trim(), &sig);
            println!("{}", output);
        }
        Some(Commands::Verify { public_key }) => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).unwrap();
            if verify::verify_operation(input.trim(), &public_key) {
                println!("valid");
            } else {
                println!("invalid");
                std::process::exit(1);
            }
        }
        None => {
            Cli::parse_from(["rad", "--help"]);
        }
    }
}
