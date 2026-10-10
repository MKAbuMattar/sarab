function render() {
  document.documentElement.dataset.theme = state.theme;
  document.documentElement.classList.toggle("translucent", state.translucent);
  const paused = state.manual === true;
  $("#pause-icon").textContent = paused ? "" : "";
  $("#pause-text").textContent = paused ? t("pause.resume") : t("pause.button");

  if (selected !== null && selected >= state.displays.length) selected = null;
  $("#all-displays").setAttribute("aria-pressed", String(selected === null));
  renderMonitors();
  renderDetail();

  renderUpdate();
  renderPresets();

  renderLibrary();

  const f = $("#settings");
  const s = state.settings;
  for (const k of [
    "pause_fullscreen",
    "per_display",
    "pause_focus",
    "pause_battery",
    "pause_power_saver",
    "pause_remote",
    "pause_vm",
  ])
    f[k].checked = s[k];
  f.per_display.disabled = !s.pause_fullscreen;
  f.fps.value = String(s.fps);
  f.pause_cpu.value = String(s.pause_cpu || 0);
  f.pause_gpu.value = String(s.pause_gpu || 0);
  f.pause_memory.value = String(s.pause_memory || 0);
  f.pause_network.value = String(s.pause_network || 0);
  f.audio_mute_others.checked = s.audio_mute_others;
  f.audio_desktop_only.checked = s.audio_desktop_only;
  f.unload_minutes.value = String(s.unload_minutes ?? 0);
  f.scaling.value = s.scaling || "cover";
  f.screensaver_minutes.value = String(s.screensaver_minutes || 0);
  f.screensaver_wallpaper.replaceChildren(
    el("option", { value: "" }, t("screensaver.each")),
    ...state.library
      .filter((w) => w.kind !== "picture")
      .map((w) => el("option", { value: w.id }, wpTitle(w))),
  );
  f.screensaver_wallpaper.value = s.screensaver_wallpaper || "";
  f.screensaver_wallpaper.disabled = !s.screensaver_minutes;
  f.span.checked = s.span;
  f.mouse_input.checked = s.mouse_input;
  f.keep_frame_on_quit.checked = s.keep_frame_on_quit;
  f.lock_screen.checked = s.lock_screen;
  f.cycle_minutes.value = String(s.cycle_minutes || 0);
  f.cycle_order.value = s.cycle_order || "order";
  f.cycle_category.replaceChildren(
    el("option", { value: "all" }, t("library.allCategories")),
    ...state.categories.map((c) => el("option", { value: c }, categoryName(c))),
  );
  f.cycle_category.value = s.cycle_category || "all";
  f.cycle_order.disabled = f.cycle_category.disabled = !s.cycle_minutes;
  f.volume.value = s.volume;
  fill(f.volume);
  f.theme.value = s.theme || "system";
  f.backdrop.value = s.backdrop || "acrylic";
  f.language.value = s.language;
  if (document.activeElement !== f.app_pause)
    f.app_pause.value = s.app_pause.join("\n");
  if (document.activeElement !== f.app_play)
    f.app_play.value = s.app_play.join("\n");
  $("#autostart").checked = state.autostart;
  $("#check-updates").checked = s.check_updates;
  $("#update-channel").value = s.update_channel === "beta" ? "beta" : "stable";
  $("#about-version").textContent = t("about.version", { v: state.version });
  $("#about-webview").textContent = state.webview;
  $("#library-dir").textContent = state.library_dir;
  document.querySelectorAll("select").forEach(combo);
}
