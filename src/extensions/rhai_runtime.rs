use std::path::Path;

use rhai::{Engine, Scope, AST};

use crate::extensions::{
    hooks::{HookEvent, HookResult},
    manifest::Manifest,
    runtime::ExtRuntime,
};

pub struct RhaiRuntime {
    engine: Engine,
    ast: AST,
    notifications: Vec<String>,
}

impl RhaiRuntime {
    pub fn new(manifest: &Manifest, script_path: &Path, vault_root: Option<&Path>) -> Result<Self, String> {
        let mut engine = Engine::new();

        // Sandbox: limit operations and call depth
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(32);
        engine.set_max_string_size(1_024 * 1_024); // 1MB
        engine.set_max_array_size(10_000);
        engine.set_max_map_size(10_000);

        // Disable dangerous built-ins
        #[cfg(not(feature = "no_std"))]
        {
            engine.disable_symbol("eval");
        }

        let notifications_shared: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let notifs = notifications_shared.clone();

        // Register mimic_notify() — extension calls this to show a message
        engine.register_fn("mimic_notify", move |msg: &str| {
            if let Ok(mut n) = notifs.lock() {
                n.push(msg.to_string());
            }
        });

        // Register fs.write only if permitted
        if manifest.has_permission("fs.write") {
            engine.register_fn("mimic_write_file", |path: &str, content: &str| -> bool {
                std::fs::write(path, content).is_ok()
            });
        }

        // Register mimic_vault_root() — path of the open vault, "" if none
        let vroot = vault_root.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        engine.register_fn("mimic_vault_root", move || -> String { vroot.clone() });

        // Register process.run only if permitted
        if manifest.has_permission("process.run") {
            engine.register_fn("mimic_run", |cmd: &str, args: rhai::Array| -> String {
                let str_args: Vec<String> = args.iter()
                    .filter_map(|v| v.clone().try_cast::<String>())
                    .collect();
                std::process::Command::new(cmd)
                    .args(&str_args)
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                    .unwrap_or_default()
            });
        }

        let code = std::fs::read_to_string(script_path)
            .map_err(|e| format!("Cannot read script: {}", e))?;

        let ast = engine.compile(&code)
            .map_err(|e| format!("Rhai compile error: {}", e))?;

        // Execute top level to register functions
        let mut scope = Scope::new();
        engine.run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| format!("Rhai exec error: {}", e))?;

        // Drain initial notifications
        let initial_notifs = notifications_shared.lock()
            .map(|mut n| n.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();

        Ok(Self {
            engine,
            ast,
            notifications: initial_notifs,
        })
    }
}

impl ExtRuntime for RhaiRuntime {
    fn call_hook(&mut self, event: &HookEvent) -> HookResult {
        let fn_name = event.name();
        let mut scope = Scope::new();

        let result: Option<String> = match event {
            HookEvent::NoteSave { path, content } | HookEvent::NoteOpen { path, content } => {
                self.engine
                    .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, fn_name, (path.clone(), content.clone()))
                    .ok()
                    .and_then(|v| v.try_cast::<String>())
            }
            HookEvent::ModeChange { from, to } => {
                self.engine
                    .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, fn_name, (from.clone(), to.clone()))
                    .ok()
                    .and_then(|v| v.try_cast::<String>())
            }
            HookEvent::MarkdownBlock { lang, code } => {
                self.engine
                    .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, fn_name, (lang.clone(), code.clone()))
                    .ok()
                    .and_then(|v| v.try_cast::<String>())
            }
            HookEvent::VaultOpen { path } => {
                self.engine
                    .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, fn_name, (path.clone(),))
                    .ok()
                    .and_then(|v| v.try_cast::<String>())
            }
        };

        match result {
            Some(text) => HookResult::Text(text),
            None => HookResult::None,
        }
    }

    fn dispatch_command(&mut self, name: &str, args: &[String]) -> Option<String> {
        let mut scope = Scope::new();
        let rhai_args: rhai::Array = args.iter().map(|s| rhai::Dynamic::from(s.clone())).collect();
        self.engine
            .call_fn::<rhai::Dynamic>(&mut scope, &self.ast, "run_command", (name.to_string(), rhai_args))
            .ok()
            .and_then(|v| v.try_cast::<String>())
    }

    fn drain_notifications(&mut self) -> Vec<String> {
        self.notifications.drain(..).collect()
    }
}
