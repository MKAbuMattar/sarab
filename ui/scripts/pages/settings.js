function readSettings() {
  const f = $("#settings");
  const lines = (v) =>
    v
      .split("\n")
      .map((x) => x.trim())
      .filter(Boolean);
  return {
    ...state.settings,
    pause_fullscreen: f.pause_fullscreen.checked,
    per_display: f.per_display.checked,
    pause_focus: f.pause_focus.checked,
    pause_battery: f.pause_battery.checked,
    pause_power_saver: f.pause_power_saver.checked,
    pause_remote: f.pause_remote.checked,
    fps: Number(f.fps.value),
    pause_cpu: Number(f.pause_cpu.value),
    pause_gpu: Number(f.pause_gpu.value),
    pause_memory: Number(f.pause_memory.value),
    pause_network: Number(f.pause_network.value),
    pause_vm: f.pause_vm.checked,
    audio_mute_others: f.audio_mute_others.checked,
    audio_desktop_only: f.audio_desktop_only.checked,
    unload_minutes: Number(f.unload_minutes.value),
    scaling: f.scaling.value,
    screensaver_minutes: Number(f.screensaver_minutes.value),
    screensaver_wallpaper: f.screensaver_wallpaper.value || null,
    span: f.span.checked,
    mouse_input: f.mouse_input.checked,
    keep_frame_on_quit: f.keep_frame_on_quit.checked,
    cycle_minutes: Number(f.cycle_minutes.value),
    cycle_order: f.cycle_order.value,
    cycle_category: f.cycle_category.value,
    volume: Number(f.volume.value),
    app_pause: lines(f.app_pause.value),
    app_play: lines(f.app_play.value),
    theme: f.theme.value,
    backdrop: f.backdrop.value,
    check_updates: $("#check-updates").checked,
    update_channel: $("#update-channel").value,
    language: f.language.value,
  };
}
