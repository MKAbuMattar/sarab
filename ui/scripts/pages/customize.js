async function openProps(display) {
  const ctls = await invoke("props", { display });
  const entries = Object.entries(ctls);
  const own = entries.map(([key, c]) => control(display, key, c));
  $("#props-body").replaceChildren(
    el("h3", {}, t("props.playback")),
    ...playbackControls(display),
    el("h3", {}, t("props.own")),
    ...(own.length ? own : [el("p", { class: "caption" }, t("props.none"))]),
  );
  $("#props-body").querySelectorAll("select").forEach(combo);
  $("#props-reset").hidden = !entries.length;
  $("#props-reset").onclick = async () => {
    try {
      await invoke("reset_props", { display });
    } catch (e) {
      showError(String(e));
      return;
    }
    $("#props").close();
    openProps(display);
  };
  if (!$("#props").open) $("#props").showModal();
}

function control(display, key, c) {
  const send = (value) =>
    invoke("set_prop", { display, key, value: String(value) }).catch((e) =>
      showError(String(e)),
    );
  const label = c.text || key;
  switch (c.type) {
    case "slider": {
      const out = el("output", { class: "caption" }, c.value);
      const input = el("input", {
        type: "range",
        min: c.min,
        max: c.max,
        step: c.step ?? 1,
        value: c.value,
      });
      requestAnimationFrame(() => fill(input));
      input.addEventListener("input", () => {
        out.textContent = input.value;
        send(input.value);
      });
      return el("label", {}, el("span", { class: "row" }, label, out), input);
    }
    case "checkbox": {
      const input = el("input", { type: "checkbox", class: "switch" });
      input.checked = !!c.value;
      input.addEventListener("change", () => send(input.checked));
      return el(
        "label",
        { class: "switch-label" },
        input,
        el("span", {}, label),
      );
    }
    case "dropdown":
    case "scalerDropdown": {
      const input = el(
        "select",
        {},
        ...(c.items || []).map((it, i) => el("option", { value: i }, it)),
      );
      input.value = c.value;
      input.addEventListener("change", () => send(input.value));
      return el("label", {}, label, input);
    }
    case "color": {
      const input = el("input", { type: "color", value: c.value });
      input.addEventListener("input", () => send(input.value));
      return el("label", {}, label, input);
    }
    case "number": {
      const input = el("input", {
        type: "number",
        min: c.min,
        max: c.max,
        step: c.step ?? 1,
        value: c.value ?? 0,
      });
      input.addEventListener("change", () => send(input.value));
      return el("label", {}, label, input);
    }
    case "password": {
      const input = el("input", {
        type: "password",
        value: c.value ?? "",
        autocomplete: "off",
      });
      input.addEventListener("change", () => send(input.value));
      return el("label", {}, label, input);
    }
    case "button":
      return el(
        "div",
        {},
        el(
          "button",
          { type: "button", onclick: () => send(true) },
          c.value || label,
        ),
      );
    case "label":
      return el("p", { class: "caption" }, c.value || label);
    default: {
      const input = el("input", { type: "text", value: c.value ?? "" });
      input.addEventListener("change", () => send(input.value));
      return el("label", {}, label, input);
    }
  }
}

function fill(r) {
  r.style.setProperty(
    "--fill",
    `${((r.value - r.min) / (r.max - r.min || 1)) * 100}%`,
  );
}
document.addEventListener("input", (e) => {
  if (e.target.type === "range") fill(e.target);
});

function playbackControls(display) {
  const d = state.displays[display];
  const w = d && state.library.find((x) => x.id === d.wallpaper);
  if (!w) return [];
  const save = (patch) =>
    run(() =>
      invoke("save_settings", { new: { ...state.settings, ...patch } }),
    );
  const map = (name, key, value) => {
    const m = { ...(state.settings[name] || {}) };
    if (value === null) delete m[key];
    else m[key] = value;
    return { [name]: m };
  };
  const rows = [];
  if (w.kind === "video" || w.kind === "gif") {
    const fit = el(
      "select",
      {},
      el("option", { value: "" }, t("playlist.same")),
      ...["cover", "contain", "fill", "none"].map((f) =>
        el("option", { value: f }, t(`fit.${f}`)),
      ),
    );
    fit.value = state.settings.display_fit?.[d.key] || "";
    fit.addEventListener("change", () =>
      save(map("display_fit", d.key, fit.value || null)),
    );
    rows.push(el("label", {}, t("props.fit"), fit));
  }
  if (w.kind === "video") {
    const speed = el(
      "select",
      {},
      ...[0.5, 0.75, 1, 1.25, 1.5, 2].map((r) =>
        el(
          "option",
          { value: r },
          r === 1 ? t("props.speedNormal") : `${r.toLocaleString(lang())}×`,
        ),
      ),
    );
    speed.value = String(state.settings.wallpaper_speed?.[w.id] ?? 1);
    speed.addEventListener("change", () => {
      const r = Number(speed.value);
      save(map("wallpaper_speed", w.id, r === 1 ? null : r));
    });
    rows.push(el("label", {}, t("props.speed"), speed));
  }
  if (w.kind !== "picture" && w.kind !== "app") {
    const mine = state.settings.wallpaper_volume?.[w.id];
    const toggle = el("input", { type: "checkbox", class: "switch" });
    toggle.checked = mine !== undefined;
    const level = el("input", {
      type: "range",
      min: 0,
      max: 100,
      step: 1,
      value: mine ?? state.settings.volume,
      "aria-label": t("props.volume"),
    });
    level.disabled = !toggle.checked;
    requestAnimationFrame(() => fill(level));
    toggle.addEventListener("change", () => {
      level.disabled = !toggle.checked;
      save(
        map(
          "wallpaper_volume",
          w.id,
          toggle.checked ? Number(level.value) : null,
        ),
      );
    });
    level.addEventListener("change", () =>
      save(map("wallpaper_volume", w.id, Number(level.value))),
    );
    rows.push(
      el(
        "label",
        { class: "switch-label" },
        toggle,
        el("span", {}, t("props.ownVolume")),
      ),
      level,
    );
  }
  if (!state.settings.span) {
    const current = state.settings.display_playlists?.[d.key] || "";
    const pick = el("select", { "data-same": "true" });
    playlistOptions(pick, current, true);
    pick.addEventListener("change", async () => {
      const v = await chooseSource(pick, current);
      if (v !== null) await setDisplayPlaylist(d.key, v);
    });
    rows.push(el("label", {}, t("displays.playlist"), pick));
  }
  return rows;
}
