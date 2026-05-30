use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};
use cpml::comparison::compare_results;
use cpml::output::console::print_result;
use cpml::pipeline::run_pipeline;

/// Locate the workspace/ directory relative to the workspace root.
fn find_workspace_dir() -> Option<PathBuf> {
    // Try ../workspace relative to the current directory (when running from cpml/)
    let candidates = [
        Path::new("../workspace"),
        Path::new("./workspace"),
        // Try relative to the binary's location
        &{
            let exe = std::env::current_exe().ok()?;
            exe.parent()?.parent()?.parent()?.join("workspace")
        },
    ];
    for c in &candidates {
        if c.is_dir() {
            return Some(c.to_path_buf());
        }
    }
    None
}

#[derive(Parser)]
#[command(
    name = "cpml",
    about = "CPML — Construction Process Modeling Language compiler"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and check a .cpml file, printing diagnostics
    Check {
        /// Path to the .cpml file
        file: PathBuf,

        /// Output format: "text" (default) or "json"
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Parse a .cpml file and dump the raw deserialized document
    Parse {
        /// Path to the .cpml file
        file: PathBuf,
    },
    /// Compare two .cpml files and highlight differences
    Compare {
        /// Path to the first .cpml file (Scenario A)
        file_a: PathBuf,
        /// Path to the second .cpml file (Scenario B)
        file_b: PathBuf,
        /// Output format: "text" (default) or "json"
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Compile a .cpml file and launch the 3D workspace
    View {
        /// Path to the .cpml file
        file: PathBuf,
        /// Port for the workspace dev server (default: 3333)
        #[arg(short, long, default_value = "3333")]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { file, format } => {
            let input = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file {:?}: {}", file, e);
                    std::process::exit(1);
                }
            };

            match run_pipeline(&input) {
                Ok(result) => {
                    match format.as_str() {
                        "json" => cpml::output::json::print_json_result(&result),
                        _ => print_result(&result),
                    }
                    let has_errors = result
                        .diagnostics
                        .iter()
                        .any(|d| d.level >= cpml::schema::DiagnosticLevel::Error);
                    if has_errors {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Compare {
            file_a,
            file_b,
            format,
        } => {
            let input_a = std::fs::read_to_string(&file_a).unwrap_or_else(|e| {
                eprintln!("Error reading file {:?}: {}", file_a, e);
                process::exit(1);
            });
            let input_b = std::fs::read_to_string(&file_b).unwrap_or_else(|e| {
                eprintln!("Error reading file {:?}: {}", file_b, e);
                process::exit(1);
            });

            let result_a = run_pipeline(&input_a).unwrap_or_else(|e| {
                eprintln!("Error processing {:?}: {}", file_a, e);
                process::exit(1);
            });
            let result_b = run_pipeline(&input_b).unwrap_or_else(|e| {
                eprintln!("Error processing {:?}: {}", file_b, e);
                process::exit(1);
            });

            let summary = compare_results(&result_a, &result_b);
            match format.as_str() {
                "json" => cpml::output::compare::print_json_comparison(&summary),
                _ => cpml::output::compare::print_comparison(&summary),
            }
        }
        Command::Parse { file } => {
            let input = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file {:?}: {}", file, e);
                    std::process::exit(1);
                }
            };

            match cpml::pipeline::parse::parse_yaml(&input) {
                Ok(doc) => {
                    println!("{:#?}", doc);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::View { file, port } => {
            let input = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file {:?}: {}", file, e);
                    std::process::exit(1);
                }
            };

            let result = match run_pipeline(&input) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error compiling {:?}: {}", file, e);
                    std::process::exit(1);
                }
            };

            // Find workspace directory: try ../workspace (from cpml/) then ./workspace
            let workspace_dir = find_workspace_dir().unwrap_or_else(|| {
                eprintln!("Error: workspace/ directory not found. Expected at ../workspace or ./workspace relative to cpml binary.");
                std::process::exit(1);
            });

            let public_dir = workspace_dir.join("public");
            std::fs::create_dir_all(&public_dir).unwrap_or_else(|e| {
                eprintln!("Error creating workspace/public/: {}", e);
                std::process::exit(1);
            });

            let data_path = public_dir.join("data.json");
            let json = serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
                eprintln!("Error serializing to JSON: {}", e);
                std::process::exit(1);
            });
            std::fs::write(&data_path, json).unwrap_or_else(|e| {
                eprintln!("Error writing data.json: {}", e);
                std::process::exit(1);
            });

            println!("Compiled {:?} → {}", file, data_path.display());
            println!("Starting workspace on http://localhost:{}", port);
            println!("Press Ctrl+C to stop.");

            // Start vite dev server in the workspace directory
            let mut child = std::process::Command::new("bun")
                .arg("run")
                .arg("dev")
                .arg("--")
                .arg("--port")
                .arg(port.to_string())
                .current_dir(&workspace_dir)
                .spawn()
                .unwrap_or_else(|e| {
                    eprintln!("Error starting workspace: {}. Is bun installed?", e);
                    eprintln!("You can also open workspace/index.html manually and load workspace/public/data.json");
                    std::process::exit(1);
                });

            // Open browser
            let url = format!("http://localhost:{}", port);
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&url).spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", &url])
                    .spawn();
            }

            let _ = child.wait();
        }
    }
}
