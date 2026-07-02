use crate::extensions::hooks::{HookEvent, HookResult};

pub trait ExtRuntime {
    fn call_hook(&mut self, event: &HookEvent) -> HookResult;
    fn dispatch_command(&mut self, name: &str, args: &[String]) -> Option<String>;
    fn drain_notifications(&mut self) -> Vec<String>;
}
