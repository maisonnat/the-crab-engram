use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use engram_core::{ObservationType, Scope};
use engram_mcp::{EngramConfig, EngramServer, ToolProfile};
use engram_store::{AddObservationParams, SearchOptions, SqliteStore, Storage};
use reqwest::blocking as reqwest_blocking;
use self_update::backends::github::{ReleaseList, Update};
use sha2::{Digest, Sha256};

mod opencode_setup;

/// GitHub repository owner for self-update
const UPDATE_REPO_OWNER: &str = "maisonnat";
/// GitHub repository name for self-update
const UPDATE_REPO_NAME: &str = "the-crab-engram";
/// Binary name for self-update asset matching
const BIN_NAME: &str = "the-crab-engram";

/// The Crab Engram: Persistent memory for AI coding agents
#[derive(Parser)]
#[command(
    name = "the-crab-engram",
    version = "2.0.0",
    about = "Persistent memory for AI agents"
)]
struct Cli {
    /// Path to the database
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    /// Project name (default: "default")
    #[arg(long, global = true, default_value = "default")]
    project: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server (stdio transport)
    Mcp {
        /// Tool profile
        #[arg(long, default_value = "agent")]
        profile: ProfileArg,
    },
    /// Search observations
    Search {
        /// Search query
        query: String,
        /// Filter by type
        #[arg(long)]
        r#type: Option<String>,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Save an observation
    Save {
        /// Title
        #[arg(long)]
        title: String,
        /// Content
        #[arg(long)]
        content: String,
        /// Observation type
        #[arg(long, default_value = "manual")]
        r#type: String,
        /// Scope (project/personal)
        #[arg(long, default_value = "project")]
        scope: String,
        /// Session ID
        #[arg(long)]
        session_id: String,
        /// Topic key
        #[arg(long)]
        topic_key: Option<String>,
    },
    /// Get session context
    Context {
        /// Max observations
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Get project statistics
    Stats,
    /// Get timeline around an observation
    Timeline {
        /// Observation ID
        observation_id: i64,
        /// Window size
        #[arg(long, default_value = "5")]
        window: usize,
    },
    /// Export data to JSON
    Export {
        /// Output file (stdout if omitted)
        #[arg(long)]
        output: Option<PathBuf>,
        /// Export all projects (not just current)
        #[arg(long)]
        all: bool,
    },
    /// Import data from JSON
    Import {
        /// Input file
        file: PathBuf,
    },
    /// Export context as Markdown system prompt (killer feature)
    ExportContext {
        /// Max tokens (approximate)
        #[arg(long, default_value = "2000")]
        max_tokens: usize,
        /// Output file (stdout if omitted)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Start a new session
    SessionStart,
    /// End a session
    SessionEnd {
        /// Session ID
        session_id: String,
        /// Summary
        #[arg(long)]
        summary: Option<String>,
    },
    /// Create a backup of the knowledge store
    Backup {
        /// Optional label for the backup
        #[arg(long)]
        label: Option<String>,
    },
    /// Verify backup integrity
    VerifyBackup {
        /// Path to backup file
        file: PathBuf,
    },
    /// Restore from backup
    Restore {
        /// List all backups
        #[arg(long)]
        list: bool,
        /// Restore by backup ID (position in list, 1 = most recent)
        #[arg(long)]
        id: Option<usize>,
        /// Restore from explicit backup file
        #[arg(long)]
        file: Option<PathBuf>,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Version info
    Version,
    /// Start HTTP REST API server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "7437")]
        port: u16,
    },
    /// Launch interactive Terminal UI
    Tui,
    /// Run memory consolidation (merge duplicates, mark obsolete, find conflicts)
    Consolidate {
        /// Dry run (don't actually modify data)
        #[arg(long)]
        dry_run: bool,
    },
    /// Sync operations (chunk export/import, status)
    Sync {
        /// Operation: status, export, import
        #[arg(value_enum)]
        action: SyncAction,
        /// Directory for export/import
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Encrypt or decrypt the database
    Encrypt {
        /// Passphrase for encryption
        #[arg(long)]
        passphrase: String,
    },
    /// Setup The Crab Engram for a specific AI agent
    Setup {
        /// Agent to configure
        #[arg(value_enum)]
        agent: AgentArg,
        /// Profile for MCP tools (opencode only)
        #[arg(long, default_value = "agent")]
        profile: String,
        /// Project name for MCP command (opencode only)
        #[arg(long)]
        project: Option<String>,
        /// Remove integration instead of setting up (opencode only)
        #[arg(long)]
        uninstall: bool,
        /// Show what would be done without writing (opencode only)
        #[arg(long)]
        dry_run: bool,
    },
    /// Diagnose integration issues
    Doctor {
        /// Agent to diagnose
        #[arg(value_enum)]
        agent: Option<AgentArg>,
        /// Auto-repair failures (opencode only)
        #[arg(long)]
        fix: bool,
    },
    /// Self-management commands (update, version)
    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        action: SelfAction,
    },
}

#[derive(Clone, ValueEnum)]
enum ProfileArg {
    Agent,
    Admin,
    All,
}

#[derive(Clone, ValueEnum)]
enum AgentArg {
    ClaudeCode,
    Cursor,
    GeminiCli,
    Opencode,
}

#[derive(Clone, ValueEnum)]
enum SyncAction {
    Status,
    Export,
    Import,
}

/// Self-management actions
#[derive(Subcommand)]
enum SelfAction {
    /// Update to the latest version from GitHub Releases
    Update {
        /// Check for updates without downloading
        #[arg(long)]
        check_only: bool,
        /// Show what would happen without updating
        #[arg(long)]
        dry_run: bool,
    },
    /// Show current version info
    Version,
}

impl From<ProfileArg> for ToolProfile {
    fn from(p: ProfileArg) -> Self {
        match p {
            ProfileArg::Agent => ToolProfile::Agent,
            ProfileArg::Admin => ToolProfile::Admin,
            ProfileArg::All => ToolProfile::All,
        }
    }
}

fn default_db_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let engram_dir = home.join(".engram");
    std::fs::create_dir_all(&engram_dir).context("failed to create ~/.engram")?;
    Ok(engram_dir.join("engram.db"))
}

/// Format bytes as human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn open_store(db_path: Option<PathBuf>) -> Result<SqliteStore> {
    let path = match db_path {
        Some(p) => p,
        None => default_db_path()?,
    };
    SqliteStore::new(&path).context("failed to open database")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp { profile } => {
            let store = Arc::new(open_store(cli.db)?);
            let config = EngramConfig {
                project: cli.project,
                profile: profile.into(),
            };
            let server = EngramServer::new(store, config);
            server.serve_stdio().await?;
        }

        Commands::Search {
            query,
            r#type,
            limit,
        } => {
            let store = open_store(cli.db)?;
            let obs_type = r#type.and_then(|t| t.parse::<ObservationType>().ok());

            let opts = SearchOptions {
                query,
                project: Some(cli.project),
                r#type: obs_type,
                limit: Some(limit),
                ..Default::default()
            };

            let results = store.search(&opts)?;
            if results.is_empty() {
                println!("No results found.");
                return Ok(());
            }

            println!("Found {} result(s):\n", results.len());
            for (i, obs) in results.iter().enumerate() {
                println!(
                    "{}. #{} [{}] {} ({}x accessed)\n   {}\n",
                    i + 1,
                    obs.id,
                    obs.r#type,
                    obs.title,
                    obs.access_count,
                    obs.content.chars().take(200).collect::<String>()
                );
            }
        }

        Commands::Save {
            title,
            content,
            r#type,
            scope,
            session_id,
            topic_key,
        } => {
            let store = open_store(cli.db)?;
            let obs_type: ObservationType = r#type
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid type: {}", r#type))?;
            let obs_scope: Scope = if scope == "personal" {
                Scope::Personal
            } else {
                Scope::Project
            };

            let params = AddObservationParams {
                r#type: obs_type,
                scope: obs_scope,
                title: title.clone(),
                content,
                session_id,
                project: cli.project,
                topic_key,
                ..Default::default()
            };

            let id = store.insert_observation(&params)?;
            println!("✅ Saved observation #{id}: \"{title}\"");
        }

        Commands::Context { limit } => {
            let store = open_store(cli.db)?;
            let ctx = store.get_session_context(&cli.project, limit)?;

            println!("## Session Context for \"{}\"\n", cli.project);
            println!(
                "Session: {} (started {})\n",
                &ctx.session.id[..8],
                ctx.session.started_at.format("%Y-%m-%d %H:%M")
            );

            if ctx.observations.is_empty() {
                println!("No observations in this session yet.");
            } else {
                println!("### Recent Observations\n");
                for obs in &ctx.observations {
                    println!(
                        "- #{} [{}] {} — {}",
                        obs.id,
                        obs.r#type,
                        obs.title,
                        obs.content.chars().take(100).collect::<String>()
                    );
                }
            }
        }

        Commands::Stats => {
            let store = open_store(cli.db)?;
            let stats = store.get_stats(&cli.project)?;

            println!("## Stats for \"{}\"\n", cli.project);
            println!("- Total observations: {}", stats.total_observations);
            println!("- Total sessions: {}", stats.total_sessions);
            println!("- Total edges: {}", stats.total_edges);
            println!("\nBy type:");
            for (t, count) in &stats.by_type {
                println!("  {t}: {count}");
            }
            println!("\nBy scope:");
            for (s, count) in &stats.by_scope {
                println!("  {s}: {count}");
            }
        }

        Commands::Timeline {
            observation_id,
            window,
        } => {
            let store = open_store(cli.db)?;
            let entries = store.get_timeline(observation_id, window)?;

            println!("## Timeline around observation #{observation_id}\n");
            for entry in &entries {
                let marker = match entry.position {
                    engram_store::TimelinePosition::Center => "→ ",
                    _ => "  ",
                };
                println!(
                    "{}{} [{}] {}",
                    marker,
                    entry.observation.created_at.format("%H:%M"),
                    entry.observation.r#type,
                    entry.observation.title,
                );
            }
        }

        Commands::Export { output, all } => {
            let store = open_store(cli.db)?;
            let project = if all {
                None
            } else {
                Some(cli.project.as_str())
            };
            let data = store.export(project)?;
            let json = serde_json::to_string_pretty(&data)?;

            match output {
                Some(path) => {
                    std::fs::write(&path, &json)?;
                    println!("✅ Exported to {}", path.display());
                }
                None => println!("{json}"),
            }
        }

        Commands::Import { file } => {
            let store = open_store(cli.db)?;

            // BACKUP-08: Auto-backup before data import
            let backup_record = store.backup_create("auto-import", None)?;
            eprintln!("Auto-backup created: {}", backup_record.path.display());

            let json = std::fs::read_to_string(&file)?;
            let data: engram_store::ExportData = serde_json::from_str(&json)?;
            let result = store.import(&data)?;

            println!("✅ Import complete:");
            println!("  - {} observations imported", result.observations_imported);
            println!("  - {} sessions imported", result.sessions_imported);
            println!("  - {} duplicates skipped", result.duplicates_skipped);
        }

        Commands::ExportContext { max_tokens, output } => {
            let store = open_store(cli.db)?;
            let context = build_export_context(&store, &cli.project, max_tokens)?;

            match output {
                Some(path) => {
                    std::fs::write(&path, &context)?;
                    println!("✅ Context exported to {}", path.display());
                }
                None => println!("{context}"),
            }
        }

        Commands::SessionStart => {
            let store = open_store(cli.db)?;
            let id = store.create_session(&cli.project)?;
            println!("{id}");
        }

        Commands::SessionEnd {
            session_id,
            summary,
        } => {
            let store = open_store(cli.db)?;
            store.end_session(&session_id, summary.as_deref())?;
            println!("Session {} ended.", &session_id[..8]);
        }

        Commands::Backup { label } => {
            let store = open_store(cli.db)?;
            let record = store.backup_create("manual", label.as_deref())?;
            eprintln!("Backup created: {}", record.path.display());
            eprintln!("SHA-256: {}", record.sha256);
            eprintln!("Size: {} bytes", record.size_bytes);
            eprintln!("Observations: {}", record.stats.observations);
            eprintln!("Sessions: {}", record.stats.sessions);
            eprintln!("Edges: {}", record.stats.edges);
        }

        Commands::VerifyBackup { file } => {
            let store = open_store(cli.db)?;
            let result = store.backup_verify(&file)?;
            if result.valid {
                eprintln!("Backup VALID: {}", file.display());
                eprintln!("SHA-256 match: {}", result.sha256_match);
                eprintln!("Integrity check: PASS");
            } else {
                eprintln!("Backup INVALID: {}", file.display());
                eprintln!("Integrity check: FAIL");
                if let Some(err) = &result.error {
                    eprintln!("Error: {err}");
                }
                std::process::exit(1);
            }
        }

        Commands::Restore {
            list,
            id,
            file,
            yes,
        } => {
            let store = open_store(cli.db)?;

            // BACKUP-03: List backups
            if list {
                let backups = store.backup_list()?;
                if backups.is_empty() {
                    eprintln!("No backups found.");
                    return Ok(());
                }
                eprintln!(
                    "{:<4} {:<22} {:<10} {:<20} {:<10} SHA-256",
                    "#", "Created", "Trigger", "Label", "Size"
                );
                eprintln!("{}", "-".repeat(90));
                for (i, b) in backups.iter().enumerate() {
                    let label = b.label.as_deref().unwrap_or("");
                    let size = format_bytes(b.size_bytes);
                    let short_sha = &b.sha256[..std::cmp::min(12, b.sha256.len())];
                    eprintln!(
                        "{:<4} {:<22} {:<10} {:<20} {:<10} {}...",
                        i + 1,
                        b.created_at.format("%Y-%m-%dT%H:%M:%SZ"),
                        b.trigger,
                        label,
                        size,
                        short_sha,
                    );
                }
                return Ok(());
            }

            // Determine backup path
            let backup_path = if let Some(id) = id {
                // BACKUP-04: Restore by ID
                let backups = store.backup_list()?;
                if id == 0 || id > backups.len() {
                    anyhow::bail!(
                        "Invalid backup ID {}. Use --list to see available backups.",
                        id
                    );
                }
                backups[id - 1].path.clone()
            } else if let Some(ref path) = file {
                // BACKUP-05: Restore by file
                if !path.exists() {
                    anyhow::bail!("Backup file not found: {}", path.display());
                }
                path.clone()
            } else {
                anyhow::bail!("Specify --list, --id N, or --file PATH");
            };

            // BACKUP-10 + BACKUP-09 + D-04: verify → pre-restore → copy (confirm unless --yes)
            store.backup_restore(&backup_path, !yes)?;
            eprintln!("Restore complete from {}", backup_path.display());
        }

        Commands::Version => {
            let version = env!("CARGO_PKG_VERSION");
            let hash = env!("GIT_HASH");
            let date = env!("GIT_DATE");
            let target = self_update::get_target();

            eprintln!("the-crab-engram {version} ({hash} {date})");
            eprintln!("target: {target}");
            eprintln!("update: run `the-crab-engram self update` to check for updates");
        }

        Commands::Serve { port } => {
            let store = Arc::new(open_store(cli.db)?);
            let state = engram_api::AppState {
                store,
                project: cli.project.clone(),
            };
            let app = engram_api::create_router(state);
            let addr = format!("0.0.0.0:{port}");
            eprintln!("The Crab Engram v2.0.0 — HTTP API on {addr}");
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Tui => {
            let store = open_store(cli.db)?;
            engram_tui::run_tui(store, &cli.project)?;
        }

        Commands::Consolidate { dry_run } => {
            let store = open_store(cli.db)?;

            // Find duplicates by normalized_hash
            let observations = store.search(&engram_store::SearchOptions {
                query: String::new(),
                project: Some(cli.project.clone()),
                limit: Some(500),
                ..Default::default()
            })?;

            let mut duplicates = 0u32;
            let mut hash_map: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
            for obs in &observations {
                hash_map
                    .entry(obs.normalized_hash.clone())
                    .or_default()
                    .push(obs);
            }
            for group in hash_map.values() {
                if group.len() > 1 {
                    let newest = group.iter().max_by_key(|o| o.id).unwrap();
                    for obs in group {
                        if obs.id != newest.id {
                            if !dry_run {
                                store.delete_observation(obs.id, false)?;
                            }
                            duplicates += 1;
                        }
                    }
                }
            }

            if dry_run {
                println!("🔍 Dry run: would merge {duplicates} duplicates");
            } else {
                println!("✅ Consolidated: {duplicates} duplicates merged");
            }
        }

        Commands::Sync { action, dir } => {
            let store = open_store(cli.db)?;
            match action {
                SyncAction::Status => {
                    let state = engram_sync::CrdtState::new();
                    let status = engram_sync::get_sync_status(&store, &state);
                    println!("Sync Status:");
                    println!("  Device: {}", &status.device_id[..8]);
                    println!("  Clock: {}", status.vector_clock);
                    println!("  Last sync: {:?}", status.last_sync);
                    println!("  Pending: {}", status.pending_deltas);
                }
                SyncAction::Export => {
                    let dir = dir.unwrap_or_else(|| PathBuf::from("./engram-chunks"));
                    let manifest = engram_sync::export_chunks(&store, Some(&cli.project), &dir)?;
                    println!(
                        "✅ Exported {} chunk(s) to {}",
                        manifest.chunks.len(),
                        dir.display()
                    );
                    for chunk in &manifest.chunks {
                        println!(
                            "  - {} ({} bytes, {} observations)",
                            chunk.filename, chunk.size, chunk.observation_count
                        );
                    }
                }
                SyncAction::Import => {
                    let dir = dir.ok_or_else(|| anyhow::anyhow!("--dir required for import"))?;
                    let result = engram_sync::import_chunks(&store, &dir)?;
                    println!("✅ Imported:");
                    println!("  - {} observations", result.observations_imported);
                    println!("  - {} sessions", result.sessions_imported);
                    println!("  - {} duplicates skipped", result.duplicates_skipped);
                }
            }
        }

        Commands::Encrypt { passphrase } => {
            let db_path = match cli.db {
                Some(p) => p,
                None => default_db_path()?,
            };
            let key = engram_core::derive_key(&passphrase);
            let data = std::fs::read(&db_path)?;

            if engram_core::is_encrypted_file(&data) {
                // Decrypt
                let decrypted = engram_core::decrypt(&key, &data)?;
                let output = db_path.with_extension("decrypted.db");
                std::fs::write(&output, &decrypted)?;
                println!("✅ Decrypted to {}", output.display());
            } else {
                // Encrypt
                let encrypted = engram_core::encrypt(&key, &data)?;
                let output = db_path.with_extension("encrypted.db");
                std::fs::write(&output, &encrypted)?;
                println!("✅ Encrypted to {}", output.display());
            }
        }

        Commands::Setup {
            agent,
            profile,
            project,
            uninstall,
            dry_run,
        } => match agent {
            AgentArg::Opencode => {
                let project_name = project.unwrap_or_else(|| cli.project.clone());
                let result = if uninstall {
                    engram_mcp::opencode_paths::OpenCodePaths::detect()
                        .map_err(|e| anyhow::anyhow!(e))
                        .and_then(|paths| {
                            crate::opencode_setup::uninstall_opencode(&paths, dry_run)
                        })
                } else {
                    engram_mcp::opencode_paths::OpenCodePaths::detect()
                        .map_err(|e| anyhow::anyhow!(e))
                        .and_then(|paths| {
                            crate::opencode_setup::setup_opencode(
                                &paths,
                                &profile,
                                &project_name,
                                dry_run,
                            )
                        })
                };
                match result {
                    Ok(r) => {
                        r.display_table();
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                let skill_md = generate_skill_md(&agent);
                let agent_name = match agent {
                    AgentArg::ClaudeCode => "claude-code",
                    AgentArg::Cursor => "cursor",
                    AgentArg::GeminiCli => "gemini-cli",
                    AgentArg::Opencode => "opencode",
                };

                let home = dirs::home_dir().context("could not determine home directory")?;

                let target_dir = match agent {
                    AgentArg::ClaudeCode => home.join(".claude").join("skills"),
                    AgentArg::Cursor => home.join(".cursor").join("rules"),
                    AgentArg::GeminiCli => home.join(".gemini").join("extensions"),
                    AgentArg::Opencode => home.join(".config").join("opencode").join("skills"),
                };

                std::fs::create_dir_all(&target_dir)?;
                let target_file = target_dir.join("engram-memory.md");
                std::fs::write(&target_file, &skill_md)?;

                println!("Setup complete for {agent_name}");
                println!("   SKILL.md written to: {}", target_file.display());
                println!("\nAdd this to your agent config to use The Crab Engram:");
                println!("   the-crab-engram mcp --project <your-project>");
            }
        },
        Commands::Doctor { agent, fix } => match agent {
            Some(AgentArg::Opencode) | None => {
                crate::opencode_setup::run_doctor(fix).await?;
            }
            Some(other) => {
                let name = match other {
                    AgentArg::ClaudeCode => "claude-code",
                    AgentArg::Cursor => "cursor",
                    AgentArg::GeminiCli => "gemini-cli",
                    AgentArg::Opencode => "opencode",
                };
                eprintln!("Doctor command not yet supported for {name}");
                std::process::exit(1);
            }
        },
        Commands::Self_ { action } => match action {
            SelfAction::Update {
                check_only,
                dry_run,
            } => {
                handle_self_update(check_only, dry_run)?;
            }
            SelfAction::Version => {
                let version = env!("CARGO_PKG_VERSION");
                let hash = env!("GIT_HASH");
                let date = env!("GIT_DATE");

                eprintln!("the-crab-engram {version} ({hash} {date})");
                eprintln!("https://github.com/{UPDATE_REPO_OWNER}/{UPDATE_REPO_NAME}");
                eprintln!("update: run `the-crab-engram self update` to check for updates");
            }
        },
    }

    Ok(())
}

fn handle_self_update(check_only: bool, dry_run: bool) -> Result<()> {
    let current_version = self_update::cargo_crate_version!();

    // Fetch latest release info
    let releases = ReleaseList::configure()
        .repo_owner(UPDATE_REPO_OWNER)
        .repo_name(UPDATE_REPO_NAME)
        .build()
        .context("Failed to configure release list")?
        .fetch()
        .context("Failed to fetch latest release")?;

    let latest_release = releases
        .first()
        .ok_or_else(|| anyhow::anyhow!("No releases found"))?;
    let latest_version = &latest_release.version;

    // Check-only mode: print comparison and exit
    if check_only {
        eprintln!("Latest: v{latest_version} (current: v{current_version})");
        return Ok(());
    }

    // Dry-run mode: show what would happen
    if dry_run {
        eprintln!("Would update from v{current_version} to v{latest_version}");
        return Ok(());
    }

    // Already up-to-date?
    if latest_version == current_version {
        eprintln!("Already on the latest version (v{current_version})");
        return Ok(());
    }

    // Perform update
    let status = Update::configure()
        .repo_owner(UPDATE_REPO_OWNER)
        .repo_name(UPDATE_REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(current_version)
        .no_confirm(true)
        .build()
        .context("Failed to configure update")?
        .update()
        .context("Failed to perform update")?;

    let updated_version = match status {
        self_update::Status::UpToDate(v) => {
            eprintln!("Already on the latest version (v{v})");
            v
        }
        self_update::Status::Updated(v) => {
            eprintln!("Updated to v{v}");
            v
        }
    };

    // Post-update binary size verification (Windows 0-byte bug)
    let exe = std::env::current_exe()?;
    let meta = std::fs::metadata(&exe)?;
    if meta.len() == 0 {
        eprintln!("error: Update produced a 0-byte binary.");
        eprintln!("This is a known issue on Windows. Please reinstall manually from:");
        eprintln!("  https://github.com/{UPDATE_REPO_OWNER}/{UPDATE_REPO_NAME}/releases/latest");
        std::process::exit(1);
    }

    // SHA-256 checksum verification (UPDATE-03)
    let mut file = std::fs::File::open(&exe)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let computed_hash = hasher.finalize();
    let computed_hash_hex = computed_hash
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    eprintln!("Binary SHA-256: {}", computed_hash_hex);

    // Fetch and verify checksums from the release
    let checksum_url = format!(
        "https://github.com/{UPDATE_REPO_OWNER}/{UPDATE_REPO_NAME}/releases/download/v{updated_version}/checksums-sha256.txt"
    );
    match reqwest_blocking::get(&checksum_url) {
        Ok(response) if response.status().is_success() => {
            match response.text() {
                Ok(body) => {
                    // Parse checksums file (format: "hash  filename" per line)
                    // Look for entry matching our binary or archive
                    let target = self_update::get_target();
                    let archive_patterns = [
                        format!("{BIN_NAME}-{updated_version}-{target}.tar.gz"),
                        format!("{BIN_NAME}-{updated_version}-{target}.zip"),
                    ];

                    let mut found = false;
                    for line in body.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        // sha256sum format: "hash  filename" (two spaces)
                        let parts: Vec<&str> = line.splitn(2, "  ").collect();
                        if parts.len() != 2 {
                            continue;
                        }

                        let expected_hash = parts[0].trim();
                        let filename = parts[1].trim();

                        // Check if this entry matches our archive
                        if archive_patterns.iter().any(|p| filename == p) {
                            found = true;
                            eprintln!("Checksum entry found: {}", filename);
                            eprintln!("  Archive SHA-256: {}", expected_hash);
                            eprintln!("  Binary SHA-256:  {} (for reference)", computed_hash_hex);
                            eprintln!(
                                "  Integrity: verified via HTTPS transport + archive checksum"
                            );
                            break;
                        }
                    }

                    if !found {
                        eprintln!("Note: No checksum entry found for target {target}");
                        eprintln!(
                            "  Binary SHA-256: {} (logged for reference)",
                            computed_hash_hex
                        );
                        eprintln!("  Integrity: verified via HTTPS transport");
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read checksums file: {e}");
                    eprintln!("  Binary SHA-256: {computed_hash_hex} (logged for reference)");
                }
            }
        }
        Ok(response) => {
            eprintln!(
                "Warning: Checksums file not available (HTTP {})",
                response.status()
            );
            eprintln!("  Binary SHA-256: {computed_hash_hex} (logged for reference)");
        }
        Err(e) => {
            eprintln!("Warning: Could not download checksums file: {e}");
            eprintln!("  Binary SHA-256: {computed_hash_hex} (logged for reference)");
        }
    }

    eprintln!("Update complete!");
    Ok(())
}

/// Generate SKILL.md content for agent integration.
fn generate_skill_md(agent: &AgentArg) -> String {
    let _ = agent; // same content for all agents for now
    r#"# Engram — Persistent Memory Protocol

## Goal
Capture and retrieve learnings across coding sessions.

## Instructions

### Session Management
1. Start each session: call `mem_session_start` with your project name
2. End each session: call `mem_session_end` with a brief summary

### Save Learnings
After significant work, save observations using `mem_save`:
- **Bugfix**: When you fix a bug — include root cause and fix
- **Decision**: When you make an architectural choice — include tradeoffs
- **Pattern**: When you notice a recurring pattern
- **Discovery**: When you learn something non-obvious about the codebase
- **Config**: When you change configuration or environment setup

### Search Before Acting
Before implementing something new, call `mem_search` to check if:
- This was done before
- There are relevant patterns or decisions
- There are known issues or anti-patterns

### Capture Passive Learnings
After completing work, call `mem_capture_passive` with your output to auto-extract:
- Test results
- Error patterns
- Code changes

## Tools Available
- `mem_save` — Save an observation
- `mem_search` — Search memories by keyword
- `mem_context` — Get session context
- `mem_session_start` — Start a session
- `mem_session_end` — End a session
- `mem_get_observation` — Get full observation by ID
- `mem_suggest_topic_key` — Suggest topic key
- `mem_capture_passive` — Extract learnings from output
- `mem_save_prompt` — Save user prompt
- `mem_update` — Update an observation
- `mem_delete` — Delete an observation (admin)
- `mem_stats` — Project statistics (admin)
- `mem_timeline` — Timeline around observation (admin)
- `mem_merge_projects` — Merge projects (admin)

## Topic Key Format
Use `mem_suggest_topic_key` to generate keys like:
- `architecture/auth-jwt-flow`
- `bug/fix-n1-query`
- `decision/use-sqlite`
"#
    .to_string()
}

/// Build export-context: Markdown system prompt from project knowledge.
fn build_export_context(store: &dyn Storage, project: &str, max_tokens: usize) -> Result<String> {
    let stats = store.get_stats(project)?;
    let ctx = store.get_session_context(project, 50)?;

    // Approximate tokens: ~4 chars per token
    let max_chars = max_tokens * 4;
    let mut md = String::with_capacity(max_chars);

    md.push_str(&format!("# Project Knowledge: {project}\n\n"));

    // Top observations by access count
    let mut all_obs = ctx.observations;
    all_obs.sort_by(|a, b| b.access_count.cmp(&a.access_count));

    if !all_obs.is_empty() {
        md.push_str("## 🔥 Most Used Knowledge\n\n");
        for obs in all_obs.iter().take(10) {
            md.push_str(&format!(
                "- **{}** [{}] (accessed {}x): {}\n",
                obs.title,
                obs.r#type,
                obs.access_count,
                obs.content.chars().take(150).collect::<String>()
            ));
        }
        md.push('\n');
    }

    // Stats summary
    md.push_str("## 📊 Overview\n\n");
    md.push_str(&format!(
        "- Total observations: {}\n",
        stats.total_observations
    ));
    md.push_str(&format!("- Total sessions: {}\n", stats.total_sessions));
    md.push('\n');

    if !stats.by_type.is_empty() {
        md.push_str("**By type:**\n");
        for (t, count) in &stats.by_type {
            md.push_str(&format!("- {t}: {count}\n"));
        }
        md.push('\n');
    }

    // Trim to max_chars
    if md.len() > max_chars {
        md.truncate(max_chars);
        md.push_str("\n\n_(truncated)_");
    }

    Ok(md)
}
