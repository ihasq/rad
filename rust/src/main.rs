use clap::Parser;
use std::io::{Read, BufRead};

mod types;
mod crypto;
mod sign;
mod verify;
mod region;
mod oplog;
mod pipeline;
mod accept;
mod reject;
mod init;
mod founder;

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
    /// Execute commands from stdin (region, write, chain)
    Pipeline,
    /// Initialize a new Rad project
    Init {
        #[arg(long)]
        participant: String,
        #[arg(long)]
        secret_key: String,
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
        Some(Commands::Init { participant, secret_key }) => {
            // Generate public key from secret key
            let kp = crypto::keypair_from_secret(&secret_key);
            let public_key = crypto::format_public_key(&kp);

            match init::init_project(std::path::Path::new("."), &participant, &public_key) {
                Ok(result) => {
                    println!("initialized: .");
                    println!("founder: {}", result.founder);
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Pipeline) => {
            let mut region_map = region::RegionMap::new();
            let mut oplog = oplog::OpLog::new();
            let mut op_ids: Vec<String> = vec![];

            // Load config.json to get root founder
            let config_path = std::path::Path::new(".rad/config.json");
            let root_founder = if config_path.exists() {
                let content = std::fs::read_to_string(config_path).unwrap();
                let config: serde_json::Value = serde_json::from_str(&content).unwrap();
                config.get("founder").and_then(|v| v.as_str()).unwrap_or("").to_string()
            } else {
                String::new()
            };

            // Load or initialize founder tree
            let founders_path = std::path::Path::new(".rad/founders.json");
            let mut founder_tree = if founders_path.exists() {
                let content = std::fs::read_to_string(founders_path).unwrap();
                founder::FounderTree::from_json(&content, &root_founder)
            } else {
                founder::FounderTree::new(&root_founder)
            };

            // Helper to expand @N references
            fn expand_refs(line: &str, op_ids: &[String]) -> String {
                let mut result = line.to_string();
                for (i, id) in op_ids.iter().enumerate() {
                    let placeholder = format!("@{}", i + 1);
                    result = result.replace(&placeholder, id);
                }
                result
            }

            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let line = line.unwrap();
                let expanded = expand_refs(&line, &op_ids);
                let parts: Vec<&str> = expanded.split_whitespace().collect();
                match parts.first().copied() {
                    Some("write") => {
                        // write <file> <start> <end> <participant> <secret-key> <text>
                        let file = parts[1];
                        let participant = parts[4];
                        founder_tree.register_from_write(file, participant);

                        let output = pipeline::handle_write(&parts, &mut region_map, &mut oplog);
                        // Extract op-id from JSON output
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
                            if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                                op_ids.push(id.to_string());
                            }
                        }
                        println!("{}", output);
                    }
                    Some("chain") => {
                        println!("{}", pipeline::handle_chain(&parts, &oplog));
                    }
                    Some("accept") => {
                        // accept <op-id> <leader> <secret-key>
                        match accept::handle_accept(parts[1], parts[2], &region_map, &mut oplog) {
                            Ok(result) => println!("{}", serde_json::to_string(&result).unwrap()),
                            Err(e) => eprintln!("error: {}", e),
                        }
                    }
                    Some("reject") => {
                        // reject <op-id> <rejecter> <secret-key> ["reason"]
                        let reason = if parts.len() > 4 {
                            Some(parts[4..].join(" ").trim_matches('"').to_string())
                        } else {
                            None
                        };
                        match reject::handle_reject(parts[1], parts[2], reason.as_deref(), &region_map, &founder_tree, &mut oplog) {
                            Ok(result) => println!("{}", serde_json::to_string(&result).unwrap()),
                            Err(e) => eprintln!("error: {}", e),
                        }
                    }
                    // region サブコマンドも pipeline 内でサポート
                    Some("region") => {
                        // region <subcommand> <args...>
                        match parts.get(1).copied() {
                            Some("register") => {
                                let r = types::CodeRegion {
                                    id: format!("{}:{}-{}", parts[2], parts[3], parts[4]),
                                    file_path: parts[2].to_string(),
                                    start_line: parts[3].parse().unwrap(),
                                    end_line: parts[4].parse().unwrap(),
                                    owner_id: parts[5].to_string(),
                                };
                                if region_map.register(r.clone()) {
                                    println!("registered: {}:{}-{} (owner: {})",
                                        r.file_path, r.start_line, r.end_line, r.owner_id);
                                } else {
                                    println!("ignored: region already registered");
                                }
                            }
                            _ => {}
                        }
                    }
                    Some("founder") => {
                        // founder [dir]
                        let dir = parts.get(1).unwrap_or(&".");
                        // Strip trailing slash for consistency
                        let dir_normalized = dir.trim_end_matches('/');
                        let dir_normalized = if dir_normalized.is_empty() { "." } else { dir_normalized };
                        match founder_tree.get_founder(dir_normalized) {
                            Some(f) => println!("{}: founder: {}", dir, f),
                            None => println!("{}: no founder", dir),
                        }
                    }
                    _ => {}
                }
            }

            // Save founder tree
            if founders_path.parent().is_some() {
                std::fs::write(founders_path, founder_tree.to_json()).ok();
            }
        }
        None => {
            Cli::parse_from(["rad", "--help"]);
        }
    }
}
