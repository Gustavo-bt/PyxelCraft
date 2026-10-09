use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;
use pumpkin_util::text::color::NamedColor;
use pumpkin_data::attributes::Attributes;
use crate::entity::attributes::{Modifier, ModifierOperation};
use crate::command::argument_builder::{ArgumentBuilder, argument, command};
use crate::command::argument_types::core::float::FloatArgumentType;
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};

const PERM: &str = "pumpkin:command.essentials";
const WALKSPEED_MOD_ID: &str = "pumpkin:walkspeed";

fn feedback(ctx: &CommandContext, label: &str, value: &str, good: bool) {
    let color = if good { NamedColor::Green } else { NamedColor::Red };
    let msg = TextComponent::text(label.to_string()).color_named(NamedColor::Gray)
        .add_child(TextComponent::text(": ".to_string()).color_named(NamedColor::Yellow))
        .add_child(TextComponent::text(value.to_string()).color_named(color));
    ctx.source.send_message(msg);
}

struct FlyExecutor;
impl CommandExecutor for FlyExecutor {
    fn execute(&self, ctx: &CommandContext) -> CommandExecutorResult {
        let Some(player) = ctx.source.as_player() else { return Ok(0); };
        let new_state = {
            let mut a = player.abilities.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let n = !a.flying;
            a.flying = n;
            a.allow_flying = n;
            n
        };
        player.send_abilities_update();
        feedback(ctx, "Fly", if new_state { "ON" } else { "OFF" }, new_state);
        Ok(1)
    }
}

struct HealExecutor;
impl CommandExecutor for HealExecutor {
    fn execute(&self, ctx: &CommandContext) -> CommandExecutorResult {
        let Some(player) = ctx.source.as_player() else { return Ok(0); };
        let max = player.living_entity.get_max_health();
        player.living_entity.health.store(max);
        player.send_health();
        feedback(ctx, "Heal", "Curado", true);
        Ok(1)
    }
}

struct FeedExecutor;
impl CommandExecutor for FeedExecutor {
    fn execute(&self, ctx: &CommandContext) -> CommandExecutorResult {
        let Some(player) = ctx.source.as_player() else { return Ok(0); };
        player.set_food_level(20);
        player.set_food_saturation(20.0);
        player.set_food_exhaustion(0.0);
        feedback(ctx, "Feed", "Alimentado", true);
        Ok(1)
    }
}

struct GodExecutor;
impl CommandExecutor for GodExecutor {
    fn execute(&self, ctx: &CommandContext) -> CommandExecutorResult {
        let Some(player) = ctx.source.as_player() else { return Ok(0); };
        let new_state = {
            let mut a = player.abilities.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let n = !a.invulnerable;
            a.invulnerable = n;
            n
        };
        player.send_abilities_update();
        feedback(ctx, "God", if new_state { "ON" } else { "OFF" }, new_state);
        Ok(1)
    }
}

struct FlySpeedExecutor;
impl CommandExecutor for FlySpeedExecutor {
    fn execute(&self, ctx: &CommandContext) -> CommandExecutorResult {
        let Some(player) = ctx.source.as_player() else { return Ok(0); };
        let mult = FloatArgumentType::get(ctx, "speed")?;
        let internal = (mult as f64 * 0.05).clamp(0.0, 0.25);
        {
            let mut a = player.abilities.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            a.fly_speed = internal as f32;
        }
        player.send_abilities_update();
        feedback(ctx, "FlySpeed", &format!("{mult:.2}x"), true);
        Ok(1)
    }
}

struct WalkSpeedExecutor;
impl CommandExecutor for WalkSpeedExecutor {
    fn execute(&self, ctx: &CommandContext) -> CommandExecutorResult {
        let Some(player) = ctx.source.as_player() else { return Ok(0); };
        let mult = FloatArgumentType::get(ctx, "speed")?;
        player.living_entity.update_attribute(&Attributes::MOVEMENT_SPEED, |speed| {
            speed.remove_modifier(WALKSPEED_MOD_ID);
            if (mult - 1.0).abs() > 0.01 {
                speed.add_or_replace_modifier(Modifier {
                    id: WALKSPEED_MOD_ID.to_string(),
                    amount: mult as f64,
                    operation: ModifierOperation::MultiplyTotal,
                });
            }
        });
        crate::entity::attributes::send_attribute_updates_for_living(
            &player.living_entity,
            vec![Attributes::MOVEMENT_SPEED],
        );
        feedback(ctx, "WalkSpeed", &format!("{mult:.2}x"), true);
        Ok(1)
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(PERM, "Essentials commands", PermissionDefault::Op(PermissionLvl::Two)));
    dispatcher.register(command("fly", "Toggle flight").requires(PERM).executes(FlyExecutor));
    dispatcher.register(command("heal", "Restore health").requires(PERM).executes(HealExecutor));
    dispatcher.register(command("feed", "Restore hunger").requires(PERM).executes(FeedExecutor));
    dispatcher.register(command("god", "Toggle invulnerability").requires(PERM).executes(GodExecutor));
    dispatcher.register(command("flyspeed", "Set fly speed (0-5, 1=normal)").requires(PERM).then(argument("speed", FloatArgumentType::new(0.0, 5.0)).executes(FlySpeedExecutor)));
    dispatcher.register(command("walkspeed", "Set walk speed (0-5, 1=normal)").requires(PERM).then(argument("speed", FloatArgumentType::new(0.0, 5.0)).executes(WalkSpeedExecutor)));
}