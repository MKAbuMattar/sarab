async function openProps(display) {
  const ctls = await invoke("props", { display });
  const entries = Object.entries(ctls);
  $("#props-body").replaceChildren(
    ...(entries.length
      ? entries.map(([key, c]) => control(display, key, c))
      : [el("p", { class: "caption" }, t("props.none"))]),
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
