use super::*;

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

pub(in crate::engine::wallpaper) fn other_load(core: &mut Core, s: &mut Signals) {
    let limits = (
        core.settings.pause_gpu,
        core.settings.pause_memory,
        core.settings.pause_network,
    );
    let gpu = if limits.0 == 0 {
        core.gpu = None;
        0.0
    } else {
        if core.gpu.is_none() {
            core.gpu = os::GpuMeter::new();
        }
        core.gpu.as_mut().and_then(|g| g.percent()).unwrap_or(0.0)
    };
    s.gpu_busy = core.gpu_gate.step(gpu, limits.0);
    let memory = if limits.1 == 0 {
        0.0
    } else {
        let (free, total) = os::memory();
        if total == 0 {
            0.0
        } else {
            total.saturating_sub(free) as f64 * 100.0 / total as f64
        }
    };
    s.memory_busy = core.memory_gate.step(memory, limits.1);
    let network = if limits.2 == 0 {
        core.net_prev = None;
        0.0
    } else {
        let (down, up) = os::net_octets();
        let now = std::time::Instant::now();
        match core.net_prev.replace((down + up, now)) {
            Some((bytes, at)) => {
                let secs = now.duration_since(at).as_secs_f64().max(0.001);
                let mbit = (down + up).saturating_sub(bytes) as f64 * 8.0 / 1e6 / secs;
                mbit * 100.0 / f64::from(limits.2)
            }
            None => 0.0,
        }
    };
    s.network_busy = core
        .network_gate
        .step(network, if limits.2 == 0 { 0 } else { 100 });
    s.virtual_machine = os::virtual_machine();
}
