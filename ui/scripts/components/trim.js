// The part of a video that plays: a preview, a track with start and end handles,
// time fields, [ and ] marks while it plays, and a loop of only the seam.
const TRIM_LAG = 0.25; // reaction time taken off a [ or ] mark made while playing
const TRIM_MIN = 0.5; // the shortest loop, as the backend allows

const fmtTime = (s) => {
  const m = Math.floor(s / 60);
  return `${m}:${(s - m * 60).toFixed(1).padStart(4, "0")}`;
};
// "1:05.5", "65.5" and "1,5" all read as seconds.
const parseTime = (text) => {
  const parts = text.trim().replace(",", ".").split(":");
  if (parts.some((p) => p === "" || isNaN(p))) return NaN;
  return parts.reduce((a, p) => a * 60 + Number(p), 0);
};

function trimEditor(src, clip) {
  const video = el("video", {
    src,
    muted: "",
    preload: "metadata",
    playsinline: "",
  });
  video.muted = true;
  const handle = (cls, label) =>
    el("div", {
      class: `handle ${cls}`,
      role: "slider",
      tabindex: "0",
      "aria-label": label,
    });
  const hStart = handle("start", t("trim.start")),
    hEnd = handle("end", t("trim.end"));
  const range = el("div", { class: "range" }),
    head = el("div", { class: "head" });
  const track = el(
    "div",
    { class: "track", dir: "ltr" },
    range,
    head,
    hStart,
    hEnd,
  );
  const field = (name, label) =>
    el(
      "label",
      {},
      el("span", {}, label),
      el("input", { type: "text", name, inputmode: "decimal", dir: "ltr" }),
    );
  const fStart = field("start", t("trim.start")),
    fEnd = field("end", t("trim.end"));
  const play = el(
    "button",
    { type: "button", class: "play" },
    icon(""),
    el("span", {}, t("trim.play")),
  );
  const seam = el(
    "button",
    { type: "button", class: "seam", "aria-pressed": "false" },
    t("trim.seam"),
  );
  const whole = el(
    "button",
    { type: "button", class: "whole" },
    t("trim.whole"),
  );
  const said = el("p", { class: "said caption", "aria-live": "polite" });
  const root = el(
    "section",
    { id: "trim", class: "trim", "aria-labelledby": "trim-title" },
    el("h3", { id: "trim-title" }, t("trim.title")),
    video,
    track,
    el("div", { class: "times" }, fStart, fEnd),
    el("div", { class: "controls" }, play, seam, whole),
    said,
    el("p", { class: "caption" }, t("trim.hint")),
  );

  let dur = 0,
    start = 0,
    end = 0,
    looping = false;

  function render() {
    if (!dur) return;
    const pct = (s) => `${(s / dur) * 100}%`;
    hStart.style.left = pct(start);
    hEnd.style.left = pct(end);
    range.style.left = pct(start);
    range.style.width = pct(end - start);
    for (const [h, v, name] of [
      [hStart, start, t("trim.start")],
      [hEnd, end, t("trim.end")],
    ]) {
      h.setAttribute("aria-valuemin", "0");
      h.setAttribute("aria-valuemax", dur.toFixed(1));
      h.setAttribute("aria-valuenow", v.toFixed(2));
      h.setAttribute("aria-valuetext", `${name}, ${fmtTime(v)}`);
    }
    if (document.activeElement !== fStart.lastChild)
      fStart.lastChild.value = fmtTime(start);
    if (document.activeElement !== fEnd.lastChild)
      fEnd.lastChild.value = fmtTime(end);
    said.textContent = t("trim.plays", {
      a: fmtTime(start),
      b: fmtTime(end),
      d: fmtTime(dur),
    });
  }
  // Moves one edge, keeps the loop at least TRIM_MIN long, and shows that frame.
  function set(which, s, show = true) {
    s = Math.min(Math.max(s, 0), dur);
    if (which === "start") start = Math.min(s, dur - TRIM_MIN);
    else end = Math.max(s, TRIM_MIN);
    if (end - start < TRIM_MIN) {
      if (which === "start") end = Math.min(dur, start + TRIM_MIN);
      else start = Math.max(0, end - TRIM_MIN);
    }
    if (show && !looping) video.currentTime = which === "start" ? start : end;
    render();
  }

  video.addEventListener("loadedmetadata", () => {
    dur = video.duration;
    start = clip ? Math.min(clip[0], dur) : 0;
    end = clip ? Math.min(clip[1], dur) : dur;
    if (end - start < TRIM_MIN) [start, end] = [0, dur];
    video.currentTime = start;
    render();
  });
  // The playhead, and the loop: the whole selection, or only its seam.
  function tick() {
    head.style.left = dur ? `${(video.currentTime / dur) * 100}%` : "0";
    if (looping) {
      const w = Math.min(2, (end - start) / 2);
      const onlySeam = seam.getAttribute("aria-pressed") === "true";
      if (video.currentTime >= end - 0.03) video.currentTime = start;
      else if (
        onlySeam &&
        video.currentTime >= start + w &&
        video.currentTime < end - w
      )
        video.currentTime = end - w;
    }
    if (!video.paused) requestAnimationFrame(tick);
  }
  video.addEventListener("play", () => requestAnimationFrame(tick));
  video.addEventListener("seeked", tick);
  function stopLoop() {
    looping = false;
    video.pause();
    play.replaceChildren(icon(""), el("span", {}, t("trim.play")));
  }
  play.onclick = () => {
    if (looping) return stopLoop();
    looping = true;
    const w = Math.min(2, (end - start) / 2);
    video.currentTime =
      seam.getAttribute("aria-pressed") === "true" ? end - w : start;
    video.play().catch(() => {});
    play.replaceChildren(icon(""), el("span", {}, t("trim.stop")));
  };
  seam.onclick = () => {
    seam.setAttribute(
      "aria-pressed",
      String(seam.getAttribute("aria-pressed") !== "true"),
    );
    if (looping) {
      stopLoop();
      play.click();
    }
  };
  whole.onclick = () => {
    start = 0;
    end = dur;
    render();
  };

  // Dragging a handle, or clicking the track to look at a moment.
  const secAt = (x) => {
    const r = track.getBoundingClientRect();
    return ((x - r.left) / r.width) * dur;
  };
  for (const [h, which] of [
    [hStart, "start"],
    [hEnd, "end"],
  ]) {
    h.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      h.focus();
      h.setPointerCapture(e.pointerId);
      stopLoop();
      const move = (ev) => set(which, secAt(ev.clientX));
      h.addEventListener("pointermove", move);
      h.addEventListener(
        "pointerup",
        () => h.removeEventListener("pointermove", move),
        { once: true },
      );
    });
    h.addEventListener("keydown", (e) => {
      const step = e.shiftKey ? 1 : 0.1;
      const now = which === "start" ? start : end;
      const to = {
        ArrowRight: now + step,
        ArrowUp: now + step,
        ArrowLeft: now - step,
        ArrowDown: now - step,
        Home: 0,
        End: dur,
      }[e.key];
      if (to === undefined) return;
      e.preventDefault();
      set(which, to);
    });
  }
  track.addEventListener("pointerdown", (e) => {
    if (dur && !looping)
      video.currentTime = Math.min(Math.max(secAt(e.clientX), 0), dur);
  });

  // Typed times: m:ss.s or seconds. A time that does not read goes back to the last one.
  for (const [f, which] of [
    [fStart.lastChild, "start"],
    [fEnd.lastChild, "end"],
  ]) {
    const commit = () => {
      const s = parseTime(f.value);
      if (isNaN(s)) render();
      else set(which, s);
      f.value = fmtTime(which === "start" ? start : end);
    };
    f.addEventListener("change", commit);
    f.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        commit();
      }
    });
  }

  // [ and ] set the start and the end where the video is, while it plays or paused.
  root.addEventListener("keydown", (e) => {
    if (e.target.tagName === "INPUT" || !dur) return;
    if (e.code !== "BracketLeft" && e.code !== "BracketRight") return;
    e.preventDefault();
    const at = Math.max(0, video.currentTime - (video.paused ? 0 : TRIM_LAG));
    const which = e.code === "BracketLeft" ? "start" : "end";
    // A start after the end, or an end before the start, swaps the two.
    const pair = which === "start" ? [at, end] : [start, at];
    [start, end] = pair[0] <= pair[1] ? pair : [pair[1], pair[0]];
    set(which, which === "start" ? start : end, false);
  });
  addEventListener("blur", () => looping && stopLoop());

  return {
    el: root,
    // Undefined keeps the saved clip, [] plays the whole video, [start, end] that part.
    value: () =>
      !dur
        ? undefined
        : start < 0.05 && end > dur - 0.05
          ? []
          : [+start.toFixed(2), +end.toFixed(2)],
    stop() {
      stopLoop();
      video.removeAttribute("src");
      video.load();
    },
  };
}
