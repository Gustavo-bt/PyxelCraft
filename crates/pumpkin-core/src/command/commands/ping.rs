use std::sync::atomic::Ordering;
use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::{TextComponent, color::NamedColor};
use crate::command::argument_builder::{ArgumentBuilder, command};
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};

const DESCRIPTION: &str = "Shows your current ping.";
const PERMISSION: &str = "pumpkin:command.ping";

struct PingExecutor;
impl CommandExecutor for PingExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let Some(player) = context.source.as_player() else { return Ok(0); };
        let ping = player.ping.load(Ordering::Relaxed);
        let color = if ping < 100 { NamedColor::Green } else if ping < 200 { NamedColor::Yellow } else { NamedColor::Red };
        let msg = TextComponent::text("Ping").color_named(NamedColor::Gray)
            .add_child(TextComponent::text(": ").color_named(NamedColor::Yellow))
            .add_child(TextComponent::text(format!("{ping}ms")).color_named(color));
        context.source.send_message(msg);
        Ok(1)
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(PERMISSION, DESCRIPTION, PermissionDefault::Op(PermissionLvl::Zero)));
    dispatcher.register(command("ping", DESCRIPTION).requires(PERMISSION).executes(PingExecutor));
}
