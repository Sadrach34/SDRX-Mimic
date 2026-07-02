use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use mlua::{Lua, StdLib, LuaOptions};

use crate::extensions::{
    hooks::{HookEvent, HookResult},
    manifest::Manifest,
    runtime::ExtRuntime,
};

pub struct LuaRuntime {
    lua: Lua,
    notifications: Rc<RefCell<Vec<String>>>,
}

impl LuaRuntime {
    pub fn new(manifest: &Manifest, script_path: &Path) -> Result<Self, String> {
        let safe_libs = StdLib::MATH | StdLib::STRING | StdLib::TABLE | StdLib::UTF8;

        // Conditionally add string/io libs only if permitted
        let libs = if manifest.has_permission("fs.write") {
            safe_libs | StdLib::IO
        } else {
            safe_libs
        };

        let lua = Lua::new_with(libs, LuaOptions::default())
            .map_err(|e| format!("Lua init error: {}", e))?;

        let notifications: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let notifs = notifications.clone();

        // Build the `mimic` API table
        let globals = lua.globals();

        let mimic_table = lua.create_table()
            .map_err(|e| format!("Lua table error: {}", e))?;

        // mimic.notify(msg) — queue a notification to show in status bar
        let notifs_clone = notifs.clone();
        let notify_fn = lua.create_function(move |_, msg: String| {
            notifs_clone.borrow_mut().push(msg);
            Ok(())
        }).map_err(|e| format!("Lua fn error: {}", e))?;
        mimic_table.set("notify", notify_fn)
            .map_err(|e| format!("Lua set error: {}", e))?;

        // mimic.register_command(name, fn) — convenience: stores in __commands table
        let lua2 = lua.clone();
        let reg_cmd = lua.create_function(move |_, (name, func): (String, mlua::Function)| {
            let cmds: mlua::Table = lua2.globals().get("__commands")
                .unwrap_or_else(|_| lua2.create_table().unwrap());
            cmds.set(name, func)?;
            lua2.globals().set("__commands", cmds)?;
            Ok(())
        }).map_err(|e| format!("Lua fn error: {}", e))?;
        mimic_table.set("register_command", reg_cmd)
            .map_err(|e| format!("Lua set error: {}", e))?;

        // mimic.on(event_name, fn) — convenience: stores in __hooks table
        let lua3 = lua.clone();
        let reg_hook = lua.create_function(move |_, (event, func): (String, mlua::Function)| {
            let hooks: mlua::Table = lua3.globals().get("__hooks")
                .unwrap_or_else(|_| lua3.create_table().unwrap());
            hooks.set(event, func)?;
            lua3.globals().set("__hooks", hooks)?;
            Ok(())
        }).map_err(|e| format!("Lua fn error: {}", e))?;
        mimic_table.set("on", reg_hook)
            .map_err(|e| format!("Lua set error: {}", e))?;

        globals.set("mimic", mimic_table)
            .map_err(|e| format!("Lua set error: {}", e))?;
        globals.set("__commands", lua.create_table().map_err(|e| e.to_string())?)
            .map_err(|e| format!("Lua set error: {}", e))?;
        globals.set("__hooks", lua.create_table().map_err(|e| e.to_string())?)
            .map_err(|e| format!("Lua set error: {}", e))?;

        // Load the extension script
        let code = std::fs::read_to_string(script_path)
            .map_err(|e| format!("Cannot read script: {}", e))?;
        lua.load(&code).exec()
            .map_err(|e| format!("Lua load error: {}", e))?;

        Ok(Self { lua, notifications })
    }
}

impl ExtRuntime for LuaRuntime {
    fn call_hook(&mut self, event: &HookEvent) -> HookResult {
        // First check __hooks table for registered handler
        let hook_name = event.name();
        let result: Option<String> = (|| {
            let hooks: mlua::Table = self.lua.globals().get("__hooks").ok()?;
            let func: mlua::Function = hooks.get(hook_name).ok()?;
            match event {
                HookEvent::NoteSave { path, content } | HookEvent::NoteOpen { path, content } => {
                    func.call::<Option<String>>((path.clone(), content.clone())).ok()?
                }
                HookEvent::ModeChange { from, to } => {
                    func.call::<Option<String>>((from.clone(), to.clone())).ok()?
                }
                HookEvent::MarkdownBlock { lang, code } => {
                    func.call::<Option<String>>((lang.clone(), code.clone())).ok()?
                }
            }
        })();

        // Also check top-level function (convention-based, e.g. `on_save`)
        let result = result.or_else(|| {
            let func: mlua::Function = self.lua.globals().get(hook_name).ok()?;
            match event {
                HookEvent::NoteSave { path, content } | HookEvent::NoteOpen { path, content } => {
                    func.call::<Option<String>>((path.clone(), content.clone())).ok()?
                }
                HookEvent::ModeChange { from, to } => {
                    func.call::<Option<String>>((from.clone(), to.clone())).ok()?
                }
                HookEvent::MarkdownBlock { lang, code } => {
                    func.call::<Option<String>>((lang.clone(), code.clone())).ok()?
                }
            }
        });

        match result {
            Some(text) => HookResult::Text(text),
            None => HookResult::None,
        }
    }

    fn dispatch_command(&mut self, name: &str, args: &[String]) -> Option<String> {
        // Check __commands table first
        let cmds: mlua::Table = self.lua.globals().get("__commands").ok()?;
        let func: mlua::Function = cmds.get(name).ok()?;
        let lua_args: mlua::MultiValue = self.lua.create_sequence_from(args.iter().cloned())
            .ok().map(|t| mlua::Value::Table(t))
            .into_iter()
            .collect::<mlua::MultiValue>();
        func.call::<Option<String>>(lua_args).ok()?
    }

    fn drain_notifications(&mut self) -> Vec<String> {
        self.notifications.borrow_mut().drain(..).collect()
    }
}
