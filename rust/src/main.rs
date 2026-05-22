use clap::Parser;
use std::io::{Read, BufRead};

mod types;
mod crypto;
mod sign;
mod verify;
mod region;

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
    /// Manage code regions (reads commands from stdin)
    Region,
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
        Some(Commands::Region) => {
            let mut map = region::RegionMap::new();
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let line = line.unwrap();
                let parts: Vec<&str> = line.split_whitespace().collect();
                match parts.first().copied() {
                    Some("register") => {
                        // register <file> <start> <end> <owner>
                        let r = types::CodeRegion {
                            id: format!("{}:{}-{}", parts[1], parts[2], parts[3]),
                            file_path: parts[1].to_string(),
                            start_line: parts[2].parse().unwrap(),
                            end_line: parts[3].parse().unwrap(),
                            owner_id: parts[4].to_string(),
                        };
                        if map.register(r.clone()) {
                            println!("registered: {}:{}-{} (owner: {})",
                                r.file_path, r.start_line, r.end_line, r.owner_id);
                        } else {
                            println!("ignored: region already registered");
                        }
                    }
                    Some("owner") => {
                        // owner <file> <line>
                        match map.get_owner(parts[1], parts[2].parse().unwrap()) {
                            Some(o) => println!("{}", o),
                            None => println!("unowned"),
                        }
                    }
                    Some("list") => {
                        // list <file>
                        for r in map.list(parts[1]) {
                            println!("{}:{}-{}\towner:{}",
                                r.file_path, r.start_line, r.end_line, r.owner_id);
                        }
                    }
                    Some("role") => {
                        // role <file> <line> <participant>
                        println!("{}", map.get_role(parts[1], parts[2].parse().unwrap(), parts[3]));
                    }
                    _ => {}
                }
            }
        }
        None => {
            Cli::parse_from(["rad", "--help"]);
        }
    }
}
