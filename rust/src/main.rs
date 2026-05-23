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
mod delete;
mod init;
mod founder;
mod store;
mod relay;
mod git;
mod cmd;
mod remote;
mod storage;

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
    Pipeline {
        #[arg(long)]
        ephemeral: bool,
    },
    /// Initialize a new Rad project
    Init {
        #[arg(long)]
        participant: String,
        #[arg(long)]
        secret_key: String,
    },
    /// Start Rad Relay HTTP server
    Relay {
        #[arg(long, default_value = "8787")]
        port: u16,
        #[arg(long, default_value = "memory")]
        storage: String,
        #[arg(long)]
        s3_endpoint: Option<String>,
        #[arg(long)]
        s3_bucket: Option<String>,
        #[arg(long)]
        s3_access_key: Option<String>,
        #[arg(long)]
        s3_secret_key: Option<String>,
        #[arg(long, default_value = "us-east-1")]
        s3_region: String,
    },
    /// Compact operation log into snapshots
    Compact,
    /// Import Git history into Rad
    Import,
    /// Export Rad accepted operations to Git
    Export,
    /// Show project status
    Status,
    /// Show operation log
    Log {
        #[arg(long)]
        participant: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Show diff between accepted and visible writes
    Diff,
    /// Clone a project from a Relay server
    Clone {
        url: String,
        #[arg(long)]
        participant: String,
        #[arg(long)]
        secret_key: String,
    },
    /// Push local operations to Relay server
    Push,
    /// Pull remote operations from Relay server
    Pull,
}

#[tokio::main]
async fn main() {
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
        Some(Commands::Pipeline { ephemeral }) => {
            // Open RadStore (only if not ephemeral)
            let cwd = std::env::current_dir().unwrap();
            let store = if !ephemeral {
                match store::RadStore::open(&cwd) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        eprintln!("error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            // Load state from store or create new
            let mut region_map;
            let mut oplog;
            let mut founder_tree;

            if let Some(ref s) = store {
                region_map = s.load_regions();
                oplog = match s.load_oplog() {
                    Ok(log) => log,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                };
                founder_tree = s.load_founders();
            } else {
                region_map = region::RegionMap::new();
                oplog = oplog::OpLog::new();
                founder_tree = founder::FounderTree::new("");
            }

            let mut op_ids: Vec<String> = vec![];

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
                        // ファイル Founder も登録（最初の write 時のみ）
                        founder_tree.register_file_founder(file, participant);

                        let output = pipeline::handle_write(&parts, &mut region_map, &mut oplog);
                        // Extract op-id from JSON output
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
                            if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                                op_ids.push(id.to_string());
                            }
                        }
                        println!("{}", output);

                        // Persist state (only if not ephemeral)
                        if let Some(ref s) = store {
                            s.save_oplog(&oplog).ok();
                            s.save_regions(&region_map).ok();
                            s.save_founders(&founder_tree).ok();
                        }
                    }
                    Some("chain") => {
                        println!("{}", pipeline::handle_chain(&parts, &oplog));
                    }
                    Some("accept") => {
                        // accept <op-id> <leader> <secret-key>
                        match accept::handle_accept(parts[1], parts[2], &region_map, &founder_tree, &mut oplog) {
                            Ok(result) => {
                                println!("{}", serde_json::to_string(&result).unwrap());
                                if let Some(ref s) = store {
                                    s.save_oplog(&oplog).ok();
                                }
                            }
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
                            Ok(result) => {
                                println!("{}", serde_json::to_string(&result).unwrap());
                                if let Some(ref s) = store {
                                    s.save_oplog(&oplog).ok();
                                }
                            }
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
                                    if let Some(ref s) = store {
                                        s.save_regions(&region_map).ok();
                                    }
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
                    Some("file-founder") => {
                        // file-founder <file-path>
                        if parts.len() < 2 {
                            eprintln!("usage: file-founder <file-path>");
                            continue;
                        }
                        let file_path = parts[1];
                        match founder_tree.get_file_founder(file_path) {
                            Some(f) => println!("{}: file-founder: {}", file_path, f),
                            None => println!("{}: no file-founder", file_path),
                        }
                    }
                    Some("delete") => {
                        // delete <file-path> <participant> <secret-key>
                        if parts.len() < 4 {
                            eprintln!("usage: delete <file-path> <participant> <secret-key>");
                            continue;
                        }
                        let file_path = parts[1];
                        let participant = parts[2];
                        let secret_key = parts[3];

                        match delete::handle_delete(file_path, participant, secret_key, &founder_tree, &mut oplog) {
                            Ok(result) => {
                                println!("{}", serde_json::to_string(&result).unwrap());
                                if let Some(ref s) = store {
                                    s.save_oplog(&oplog).ok();
                                }
                            }
                            Err(e) => eprintln!("error: {}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
        Some(Commands::Relay {
            port,
            storage,
            s3_endpoint,
            s3_bucket,
            s3_access_key,
            s3_secret_key,
            s3_region
        }) => {
            use crate::storage::{S3Backend, S3Config, S3RadStore};

            let state = if storage == "s3" {
                // Validate S3 options
                if s3_endpoint.is_none() || s3_bucket.is_none() || s3_access_key.is_none() || s3_secret_key.is_none() {
                    eprintln!("error: S3 storage requires --s3-endpoint, --s3-bucket, --s3-access-key, and --s3-secret-key");
                    std::process::exit(1);
                }

                let config = S3Config {
                    endpoint: s3_endpoint.unwrap(),
                    bucket: s3_bucket.unwrap(),
                    access_key: s3_access_key.unwrap(),
                    secret_key: s3_secret_key.unwrap(),
                    region: s3_region,
                };

                match S3Backend::new(config) {
                    Ok(backend) => {
                        let store = std::sync::Arc::new(S3RadStore::new(std::sync::Arc::new(backend)));
                        println!("rad relay using S3 storage");

                        // Load existing data from S3
                        match relay::state::RelayState::from_s3_store(store).await {
                            Ok(state) => std::sync::Arc::new(state),
                            Err(e) => {
                                eprintln!("error: Failed to load data from S3: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("error: Failed to initialize S3 backend: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                std::sync::Arc::new(relay::state::RelayState::new())
            };

            let app = relay::create_relay_router(state);
            let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
            println!("rad relay listening on port {}", port);

            // Create socket with SO_REUSEADDR to allow quick port reuse in tests
            let socket = socket2::Socket::new(
                socket2::Domain::IPV4,
                socket2::Type::STREAM,
                Some(socket2::Protocol::TCP),
            ).unwrap();
            socket.set_reuse_address(true).unwrap();
            socket.set_nonblocking(true).unwrap();
            socket.bind(&addr.into()).unwrap();
            socket.listen(1024).unwrap();

            let listener = tokio::net::TcpListener::from_std(socket.into()).unwrap();
            axum::serve(listener, app).await.unwrap();
        }
        Some(Commands::Compact) => {
            let cwd = std::env::current_dir().unwrap();
            match store::RadStore::open(&cwd) {
                Ok(store) => {
                    match store.compact() {
                        Ok(_) => println!("compacted"),
                        Err(e) => {
                            eprintln!("error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Import) => {
            let cwd = std::env::current_dir().unwrap();
            match git::import::import_from_git(&cwd) {
                Ok(result) => {
                    println!("imported: {} commits → {} operations", result.commit_count, result.operation_count);
                    println!("participants: {} registered", result.participant_count);
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Export) => {
            let cwd = std::env::current_dir().unwrap();
            match git::export::export_to_git(&cwd) {
                Ok(result) => {
                    println!("exported: {} operations → {} commits", result.operation_count, result.commit_count);
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Status) => {
            let cwd = std::env::current_dir().unwrap();
            match store::RadStore::open(&cwd) {
                Ok(store) => {
                    match cmd::status::run_status(&store) {
                        Ok(output) => print!("{}", output),
                        Err(e) => {
                            eprintln!("error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Log { participant, file, status }) => {
            let cwd = std::env::current_dir().unwrap();
            match store::RadStore::open(&cwd) {
                Ok(store) => {
                    let opts = cmd::log::LogOptions {
                        participant,
                        file,
                        status,
                    };
                    match cmd::log::run_log(&store, &opts) {
                        Ok(output) => print!("{}", output),
                        Err(e) => {
                            eprintln!("error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Diff) => {
            let cwd = std::env::current_dir().unwrap();
            match store::RadStore::open(&cwd) {
                Ok(store) => {
                    match cmd::diff::run_diff(&store) {
                        Ok(output) => print!("{}", output),
                        Err(e) => {
                            eprintln!("error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Clone { url, participant, secret_key }) => {
            match cmd::clone::run_clone(&url, &participant, &secret_key) {
                Ok(output) => print!("{}", output),
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Push) => {
            let cwd = std::env::current_dir().unwrap();
            match store::RadStore::open(&cwd) {
                Ok(store) => {
                    match cmd::push::run_push(&store) {
                        Ok(output) => print!("{}", output),
                        Err(e) => {
                            eprintln!("error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Pull) => {
            let cwd = std::env::current_dir().unwrap();
            match store::RadStore::open(&cwd) {
                Ok(store) => {
                    match cmd::pull::run_pull(&store) {
                        Ok(output) => print!("{}", output),
                        Err(e) => {
                            eprintln!("error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            Cli::parse_from(["rad", "--help"]);
        }
    }
}
