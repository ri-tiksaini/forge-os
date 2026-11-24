#![forbid(unsafe_code)]

//! Thin CLI client of `forged` (Blueprint 6.6). Zero business logic.

use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process;

use forge_proto::{escape_json, extract_str, extract_u64, Request, Response};

fn socket_path() -> String {
    env::var("FORGE_SOCKET").unwrap_or_else(|_| forge_proto::DEFAULT_SOCKET_PATH.to_string())
}

fn rpc(method: &str, params_json: &str) -> Result<String, (i32, String)> {
    let sock = socket_path();
    let mut stream = UnixStream::connect(&sock)
        .map_err(|e| (5, format!("cannot connect to forged at {sock}: {e}")))?;
    let req = Request {
        id: 1,
        method: method.to_string(),
        params_json: params_json.to_string(),
    };
    writeln!(stream, "{}", req.to_line()).map_err(|e| (5, e.to_string()))?;
    stream.flush().map_err(|e| (5, e.to_string()))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| (5, e.to_string()))?;

    match Response::parse_line(&line) {
        Ok(Response::Ok { result_json, .. }) => Ok(result_json),
        Ok(Response::Err { error, .. }) => {
            let exit = match error.code {
                1007 => 3,
                1003 => 4,
                1005 => 5,
                _ => 1,
            };
            Err((exit, error.message))
        }
        Err(e) => Err((5, e.to_string())),
    }
}

fn print_help() {
    println!("Forge OS dev CLI (Blueprint 6.6)");
    println!("  dev search <query>      Search the software catalog");
    println!("  dev add <id...> [--yes] Install tools or stacks (shows cost first)");
    println!("  dev new <stack> <name>  Scaffold project (web|rust|python) with mise.toml");
    println!("  dev remove <id...>      Remove installed catalog entries");
    println!("  dev list [--parked]     Show installed and parked services");
    println!("  dev on <id> / off <id>  Start or park an on-demand service");
    println!("  dev doctor              Hardware tier report & daemon health");
    println!("  dev help                Show this screen");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let json_mode = args.iter().any(|a| a == "--json");
    let yes_mode = args.iter().any(|a| a == "--yes" || a == "-y");
    let positional: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(String::as_str)
        .collect();

    let Some(&cmd) = positional.first() else {
        print_help();
        return;
    };

    match cmd {
        "help" => print_help(),
        "new" => {
            let Some(&stack) = positional.get(1) else {
                eprintln!("Usage: dev new <web|rust|python> <project-name>");
                process::exit(2);
            };
            let Some(&name) = positional.get(2) else {
                eprintln!("Usage: dev new <web|rust|python> <project-name>");
                process::exit(2);
            };
            if name.contains('/') || name.contains("..") {
                eprintln!("Invalid project name: {name}");
                process::exit(2);
            }
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let dir = std::path::PathBuf::from(home).join("Projects").join(name);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                eprintln!("Failed to create {}: {e}", dir.display());
                process::exit(1);
            }
            let mise_toml = match stack {
                "web" => "[tools]\nnode = \"lts\"\n",
                "rust" => "[tools]\nrust = \"stable\"\n",
                _ => "[tools]\npython = \"3.12\"\n",
            };
            let _ = std::fs::write(dir.join("mise.toml"), mise_toml);
            let _ = std::fs::write(
                dir.join("README.md"),
                format!("# {name}\n\nScaffolded with Forge OS (`dev new {stack} {name}`).\n"),
            );
            let _ = std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(&dir)
                .status();
            println!("Created {stack} project at {}", dir.display());
            println!("Also in Settings > Projects");
        }
        "doctor" => {
            let report = forge_tier::detect();
            if json_mode {
                println!("{}", report.to_json());
            } else {
                println!("Tier       : {} (score {}/100)", report.tier, report.score);
                println!("Compositor : {}", report.tier.compositor());
                match rpc("system.state", "{}") {
                    Ok(s) => println!("forged     : online ({s})"),
                    Err((_, e)) => println!("forged     : offline ({e})"),
                }
            }
        }
        "search" => {
            let query = positional.get(1).copied().unwrap_or("");
            let params = format!("{{\"query\":\"{}\"}}", escape_json(query));
            match rpc("catalog.search", &params) {
                Ok(res) => {
                    println!("{res}");
                    if !json_mode {
                        println!("Also in Settings > Software");
                    }
                }
                Err((code, msg)) => {
                    eprintln!("Error: {msg}");
                    process::exit(code);
                }
            }
        }
        "add" | "remove" => {
            let ids: Vec<String> = positional[1..].iter().map(|s| (*s).to_string()).collect();
            if ids.is_empty() {
                eprintln!("Usage: dev {cmd} <id...>");
                process::exit(2);
            }
            let ids_json: Vec<String> = ids
                .iter()
                .map(|s| format!("\"{}\"", escape_json(s)))
                .collect();
            let remove = cmd == "remove";
            let plan_params = format!("{{\"ids\":[{}],\"remove\":{remove}}}", ids_json.join(","));

            let plan_res = match rpc("txn.plan", &plan_params) {
                Ok(r) => r,
                Err((code, msg)) => {
                    eprintln!("Error: {msg}");
                    process::exit(code);
                }
            };

            let token = extract_str(&plan_res, "token").unwrap_or_default();
            let disk = extract_u64(&plan_res, "disk_mb").unwrap_or(0);
            let ram = extract_u64(&plan_res, "peak_ram_mb").unwrap_or(0);

            if !yes_mode && !json_mode {
                print!("Plan {token}: disk ~{disk} MB, peak RAM ~{ram} MB. Proceed? [y/N] ");
                let _ = io::stdout().flush();
                let mut answer = String::new();
                let _ = io::stdin().read_line(&mut answer);
                if !answer.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return;
                }
            }

            let apply_params = format!("{{\"token\":\"{}\"}}", escape_json(&token));
            match rpc("txn.apply", &apply_params) {
                Ok(res) => {
                    println!("{res}");
                    if !json_mode {
                        println!("Also in Settings > Software");
                    }
                }
                Err((code, msg)) => {
                    eprintln!("Error: {msg}");
                    process::exit(code);
                }
            }
        }
        "list" => match rpc("system.state", "{}") {
            Ok(res) => {
                println!("{res}");
                if !json_mode {
                    println!("Also in Settings > Services");
                }
            }
            Err((code, msg)) => {
                eprintln!("Error: {msg}");
                process::exit(code);
            }
        },
        "on" | "off" => {
            let Some(&id) = positional.get(1) else {
                eprintln!("Usage: dev {cmd} <id>");
                process::exit(2);
            };
            let method = if cmd == "on" {
                "service.enable"
            } else {
                "service.disable"
            };
            let params = format!("{{\"id\":\"{}\"}}", escape_json(id));
            match rpc(method, &params) {
                Ok(res) => {
                    println!("{res}");
                    if !json_mode {
                        println!("Also in Settings > Services");
                    }
                }
                Err((code, msg)) => {
                    eprintln!("Error: {msg}");
                    process::exit(code);
                }
            }
        }
        other => {
            eprintln!("Unknown command: {other}");
            print_help();
            process::exit(2);
        }
    }
}
