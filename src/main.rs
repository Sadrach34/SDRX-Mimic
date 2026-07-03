mod app;
mod config;
mod events;
mod extensions;
mod modes;
mod ui;
mod vault;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute};

use crate::{app::App, config::Config, extensions::ExtensionManager};

#[derive(Parser)]
#[command(name = "mmc", about = "SDRX Mimic — TUI vault editor con soporte de extensiones")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Ruta al vault (sin subcomando abre el vault directamente)
    vault: Option<PathBuf>,

    /// Crear un nuevo vault en la ruta dada
    #[arg(long)]
    new: bool,

    /// Crear nota rápida sin abrir TUI: mimic <vault> --note "mensaje"
    #[arg(long, value_name = "MESSAGE")]
    note: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Gestión de extensiones
    Ext {
        #[command(subcommand)]
        action: ExtAction,
    },
}

#[derive(clap::Args)]
struct VaultScope {
    /// Operar sobre las extensiones de este vault en vez de las globales
    #[arg(long, value_name = "PATH")]
    vault: Option<PathBuf>,
}

#[derive(Subcommand)]
enum ExtAction {
    /// Instalar extensión desde una carpeta local
    Install {
        path: PathBuf,
        #[command(flatten)]
        scope: VaultScope,
    },
    /// Desinstalar una extensión por nombre
    Remove {
        name: String,
        #[command(flatten)]
        scope: VaultScope,
    },
    /// Listar extensiones instaladas
    List {
        #[command(flatten)]
        scope: VaultScope,
    },
    /// Activar una extensión
    Enable {
        name: String,
        #[command(flatten)]
        scope: VaultScope,
    },
    /// Desactivar una extensión
    Disable {
        name: String,
        #[command(flatten)]
        scope: VaultScope,
    },
}

fn resolve_manager(scope: &VaultScope) -> ExtensionManager {
    match &scope.vault {
        Some(v) => ExtensionManager::new_for_vault(v),
        None => ExtensionManager::new(),
    }
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    // Handle `mimic ext ...` subcommand (non-TUI)
    if let Some(Commands::Ext { action }) = cli.command {
        return handle_ext_command(action);
    }

    let config = Config::load();

    // Modo headless: crear nota rápida y salir sin abrir TUI
    if let Some(message) = &cli.note {
        let vault_path = cli.vault.as_deref().unwrap_or_else(|| Path::new("."));
        return create_quick_note(vault_path, message);
    }

    let mut terminal = ratatui::init();
    let _ = execute!(std::io::stdout(), EnableMouseCapture);

    let app = if let Some(path) = cli.vault {
        if cli.new {
            std::fs::create_dir_all(&path)?;
        }
        let path_str = path.to_string_lossy().to_string();
        let mut cfg = config;
        cfg.settings.vault.add_recent(&path_str);
        cfg.save();
        App::new_with_vault(&path, cfg)?
    } else if let Some(last) = config.settings.vault.recent.first().cloned() {
        let path = PathBuf::from(&last);
        if path.exists() {
            let mut cfg = config;
            cfg.settings.vault.add_recent(&last);
            App::new_with_vault(&path, cfg)?
        } else {
            App::new_at_home(config)
        }
    } else {
        App::new_at_home(config)
    };

    let result = app.run(&mut terminal);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn handle_ext_command(action: ExtAction) -> std::io::Result<()> {
    match action {
        ExtAction::List { scope } => {
            let mut manager = resolve_manager(&scope);
            manager.load_all();
            if manager.extensions.is_empty() {
                println!("Sin extensiones instaladas.");
            } else {
                println!("Extensiones instaladas:");
                for e in &manager.extensions {
                    let status = if e.manifest.enabled { "activa" } else { "inactiva" };
                    println!("  {} v{} [{}] — {}", e.manifest.name, e.manifest.version, status, e.manifest.description);
                }
            }
        }
        ExtAction::Install { path, scope } => {
            let mut manager = resolve_manager(&scope);
            manager.load_all();
            println!("⚠  ADVERTENCIA: Las extensiones son código de terceros no revisado.");
            println!("   Instala solo extensiones de fuentes confiables.");
            println!("   El creador de SDRX Mimic no se responsabiliza de daños.");
            println!();
            match manager.install_from(&path) {
                Ok(name) => println!("Extensión '{}' instalada (desactivada por defecto).\nActívala con: mmc ext enable {}", name, name),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        ExtAction::Remove { name, scope } => {
            let mut manager = resolve_manager(&scope);
            manager.load_all();
            match manager.remove(&name) {
                Ok(_) => println!("Extensión '{}' eliminada.", name),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        ExtAction::Enable { name, scope } => {
            let mut manager = resolve_manager(&scope);
            manager.load_all();
            println!("⚠  ADVERTENCIA: Activar una extensión ejecuta código de terceros.");
            println!("   El creador de SDRX Mimic no se responsabiliza de daños.");
            if let Some(idx) = manager.extensions.iter().position(|e| e.manifest.name == name) {
                manager.enable(idx);
                println!("Extensión '{}' activada.", name);
            } else {
                eprintln!("Extensión '{}' no encontrada.", name);
            }
        }
        ExtAction::Disable { name, scope } => {
            let mut manager = resolve_manager(&scope);
            manager.load_all();
            if let Some(idx) = manager.extensions.iter().position(|e| e.manifest.name == name) {
                manager.disable(idx);
                println!("Extensión '{}' desactivada.", name);
            } else {
                eprintln!("Extensión '{}' no encontrada.", name);
            }
        }
    }
    Ok(())
}

fn create_quick_note(vault_path: &Path, message: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(vault_path)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = vault_path.join(format!("{}.md", ts));
    std::fs::write(&path, format!("# Nota rápida\n\n{}\n", message))?;
    println!("Nota creada: {}", path.display());
    Ok(())
}
