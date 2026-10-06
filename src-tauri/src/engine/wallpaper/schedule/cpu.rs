//! Rest while other apps keep the CPU busy.

use super::*;

/// Are other apps keeping the CPU busy? Sarab's own processes are taken out of the reading,
/// so a heavy wallpaper never pauses itself. Measured only while the setting is on.
pub(in crate::engine::wallpaper) fn cpu_busy(core: &mut Core) -> bool {
    let limit = core.settings.pause_cpu;
    if limit == 0 {
        core.cpu_prev = None;
        return core.cpu_gate.step(0.0, 0);
    }
    let (idle, busy) = os::cpu_times();
    let own = os::own_cpu_time();
    let percent = match core.cpu_prev.replace((idle, busy, own)) {
        Some((i0, b0, o0)) => {
            let total = idle.saturating_sub(i0) + busy.saturating_sub(b0);
            let others = busy
                .saturating_sub(b0)
                .saturating_sub(own.saturating_sub(o0));
            if total == 0 {
                0.0
            } else {
                others as f64 * 100.0 / total as f64
            }
        }
        None => 0.0,
    };
    core.cpu_gate.step(percent, limit)
}
