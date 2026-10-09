use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::path::PathBuf;
use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;
use pumpkin_util::text::color::NamedColor;
use crate::command::argument_builder::{ArgumentBuilder, command};
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::server::Server;
use crate::world::bossbar::{Bossbar, BossbarColor, BossbarDivisions};
use uuid::Uuid;

pub const PERF_PERMISSION: &str = "pumpkin:command.perfbar";
const PERSIST_FILE: &str = "perf_bars.json";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PerfBarKind { Tps, Ram }
impl PerfBarKind {
    fn as_str(self) -> &'static str { match self { Self::Tps => "tps", Self::Ram => "ram" } }
    fn from_str(s: &str) -> Option<Self> { match s { "tps" => Some(Self::Tps), "ram" => Some(Self::Ram), _ => None } }
}

pub struct PerfBarEntry {
    pub player_uuid: Uuid,
    pub kind: PerfBarKind,
    pub bossbar: Bossbar,
    pub pending_show: bool,
}

// TPS text color: >=19 green, >=15 yellow, <15 red
fn tps_text_color(tps: f64) -> NamedColor {
    if tps >= 19.0 { NamedColor::Green } else if tps >= 15.0 { NamedColor::Yellow } else { NamedColor::Red }
}
// MSPT text color: <40 green, <50 yellow, else red
fn mspt_text_color(mspt: f64) -> NamedColor {
    if mspt < 40.0 { NamedColor::Green } else if mspt < 50.0 { NamedColor::Yellow } else { NamedColor::Red }
}
// Ping text color: <100 green, <200 yellow, else red
fn ping_text_color(ping: u32) -> NamedColor {
    if ping < 100 { NamedColor::Green } else if ping < 200 { NamedColor::Yellow } else { NamedColor::Red }
}

// Bar color follows MSPT thresholds (progressFillMode = MSPT)
fn tps_bar_color(mspt: f64) -> BossbarColor {
    if mspt < 40.0 { BossbarColor::Green } else if mspt < 50.0 { BossbarColor::Yellow } else { BossbarColor::Red }
}

// RAM bar color: <50% green, <75% yellow, else red
fn ram_bar_color(percent: f32) -> BossbarColor {
    if percent < 0.5 { BossbarColor::Green } else if percent < 0.75 { BossbarColor::Yellow } else { BossbarColor::Red }
}
// RAM text color: <60% green, <85% yellow, else red
fn ram_text_color(percent: f32) -> NamedColor {
    if percent < 0.6 { NamedColor::Green } else if percent < 0.85 { NamedColor::Yellow } else { NamedColor::Red }
}

pub fn load_persisted_entries() -> Vec<PerfBarEntry> {
    return Vec::new(); // DISABLED: causing reconnect crash
    #[allow(unreachable_code)]
    let path = PathBuf::from(PERSIST_FILE);
    if !path.exists() { return Vec::new(); }
    let Ok(content) = std::fs::read_to_string(&path) else { return Vec::new(); };
    let Ok(data) = serde_json::from_str::<Vec<[String; 2]>>(&content) else { return Vec::new(); };
    let mut entries = Vec::new();
    for [uuid_str, kind_str] in data {
        let Ok(uuid) = Uuid::parse_str(&uuid_str) else { continue; };
        let Some(kind) = PerfBarKind::from_str(&kind_str) else { continue; };
        let mut bossbar = Bossbar::new(TextComponent::text("..."));
        bossbar.color = BossbarColor::Green;
        bossbar.health = 0.0;
        entries.push(PerfBarEntry { player_uuid: uuid, kind, bossbar, pending_show: false });
    }
    entries
}

fn save_persisted(entries: &[PerfBarEntry]) {
    let data: Vec<[String; 2]> = entries.iter().map(|e| [e.player_uuid.to_string(), e.kind.as_str().to_string()]).collect();
    if let Ok(json) = serde_json::to_string(&data) { let _ = std::fs::write(PERSIST_FILE, json); }
}

pub fn update_perf_bars(server: &Arc<Server>) {
    let mut entries = match server.perf_bars.lock() { Ok(g) => g, Err(p) => p.into_inner() };
    if entries.is_empty() { return; }
    let max_tps = server.basic_config.tps as f64;
    let tps = server.get_tps().min(max_tps);
    let mspt = server.get_mspt();
    let used_bytes = pumpkin_process_memory_bytes();
    let total_bytes = system_total_memory_bytes();
    let percent = if total_bytes > 0 { (used_bytes as f64 / total_bytes as f64) as f32 } else { 0.0 };

    for entry in entries.iter_mut() {
        let Some(player) = server.get_player_by_uuid(entry.player_uuid) else { continue; };
        if entry.pending_show {
            player.send_bossbar(&entry.bossbar);
            entry.pending_show = false;
        }
        match entry.kind {
            PerfBarKind::Tps => {
                let ping = player.ping.load(Ordering::Relaxed);
                // progressFillMode = MSPT (plugin default)
                let tps_bad = ((20.0 - tps) / 20.0).clamp(0.0, 1.0) as f32;
                let ping_bad = (ping as f32 / 200.0).clamp(0.0, 1.0);
                let mspt_bad = (mspt as f32 / 50.0).clamp(0.0, 1.0);
                let fill = tps_bad.max(ping_bad).max(mspt_bad);
                let bar_color = if tps >= 19.0 && ping < 100 && mspt < 40.0 { BossbarColor::Green } else if tps >= 15.0 && ping < 200 && mspt < 50.0 { BossbarColor::Yellow } else { BossbarColor::Red };
                let title = TextComponent::text("TPS").color_named(NamedColor::Gray)
                    .add_child(TextComponent::text(":").color_named(NamedColor::Yellow))
                    .add_child(TextComponent::text(" "))
                    .add_child(TextComponent::text(format!("{tps:.2}")).color_named(tps_text_color(tps)))
                    .add_child(TextComponent::text(" MSPT").color_named(NamedColor::Gray))
                    .add_child(TextComponent::text(":").color_named(NamedColor::Yellow))
                    .add_child(TextComponent::text(" "))
                    .add_child(TextComponent::text(format!("{mspt:.2}")).color_named(mspt_text_color(mspt)))
                    .add_child(TextComponent::text(" Ping").color_named(NamedColor::Gray))
                    .add_child(TextComponent::text(":").color_named(NamedColor::Yellow))
                    .add_child(TextComponent::text(" "))
                    .add_child(TextComponent::text(format!("{ping}ms")).color_named(ping_text_color(ping)));
                player.update_bossbar_health(&entry.bossbar.uuid, fill);
                player.update_bossbar_title(&entry.bossbar.uuid, title);
                player.update_bossbar_style(&entry.bossbar.uuid, bar_color, BossbarDivisions::Notches20);
            }
            PerfBarKind::Ram => {
                let pct_int = (percent * 100.0) as i32;
                let used_str = format_bytes(used_bytes);
                let total_str = format_bytes(total_bytes);
                let bar_color = ram_bar_color(percent);
                let text_color = ram_text_color(percent);
                let title = TextComponent::text("Ram").color_named(NamedColor::Gray)
                    .add_child(TextComponent::text(":").color_named(NamedColor::Yellow))
                    .add_child(TextComponent::text(" "))
                    .add_child(TextComponent::text(format!("{used_str}/{total_str} ({pct_int}%)")).color_named(text_color));
                player.update_bossbar_health(&entry.bossbar.uuid, percent);
                player.update_bossbar_title(&entry.bossbar.uuid, title);
                player.update_bossbar_style(&entry.bossbar.uuid, bar_color, BossbarDivisions::Notches20);
            }
        }
    }
}

pub fn on_player_join(server: &Arc<Server>, player_uuid: Uuid) {
    let mut entries = match server.perf_bars.lock() { Ok(g) => g, Err(p) => p.into_inner() };
    for entry in entries.iter_mut() {
        if entry.player_uuid == player_uuid { entry.pending_show = true; }
    }
}

fn format_bytes(b: u64) -> String { if b >= 1073741824 { format!("{:.1}GB", b as f64 / 1073741824.0) } else { format!("{:.1}MB", b as f64 / 1048576.0) } }
fn pumpkin_process_memory_bytes() -> u64 {
    let mut sys = sysinfo::System::new();
    if let Ok(pid) = sysinfo::get_current_pid() {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        if let Some(proc) = sys.process(pid) { return proc.memory(); }
    }
    0
}

fn system_total_memory_bytes() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory()
}

fn toggle_bar(context: &CommandContext, kind: PerfBarKind) {
    let Some(player) = context.source.as_player() else { return; };
    let server = context.source.server();
    let uuid = player.gameprofile.id;
    let mut entries = match server.perf_bars.lock() { Ok(g) => g, Err(p) => p.into_inner() };
    if let Some(idx) = entries.iter().position(|e| e.player_uuid == uuid && e.kind == kind) {
        let entry = entries.remove(idx);
        player.remove_bossbar(entry.bossbar.uuid);
        save_persisted(&entries);
        return;
    }
    let mut bossbar = Bossbar::new(TextComponent::text("..."));
    bossbar.color = BossbarColor::Green;
    bossbar.health = 0.0;
    player.send_bossbar(&bossbar);
    entries.push(PerfBarEntry { player_uuid: uuid, kind, bossbar, pending_show: false });
    save_persisted(&entries);
}

struct TpsBarExecutor;
impl CommandExecutor for TpsBarExecutor {
    fn execute(&self, c: &CommandContext) -> CommandExecutorResult { toggle_bar(c, PerfBarKind::Tps); Ok(1) }
}
struct RamBarExecutor;
impl CommandExecutor for RamBarExecutor {
    fn execute(&self, c: &CommandContext) -> CommandExecutorResult { toggle_bar(c, PerfBarKind::Ram); Ok(1) }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(PERF_PERMISSION, "Toggles the performance bossbar", PermissionDefault::Op(PermissionLvl::Two)));
    dispatcher.register(command("tpsbar", "Toggles the TPS bossbar").requires(PERF_PERMISSION).executes(TpsBarExecutor));
    dispatcher.register(command("rambar", "Toggles the RAM bossbar").requires(PERF_PERMISSION).executes(RamBarExecutor));
}
