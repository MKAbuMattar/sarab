// Fills every download link from the newest GitHub release, picks the tab for this system,
// and marks the file for this PC. Without it, every link opens the latest release page.
const REPO = "MKAbuMattar/sarab";
const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];
const mb = (bytes) => `${(bytes / 1048576).toFixed(1)} MB`;
const day = (iso) =>
  new Date(iso).toLocaleDateString("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  });

// Tabs: Windows and Linux.
function show(system) {
  for (const tab of $$('[role="tab"]')) {
    const on = tab.id === `tab-${system}`;
    tab.setAttribute("aria-selected", on);
    tab.tabIndex = on ? 0 : -1;
    $(`#${tab.getAttribute("aria-controls")}`).hidden = !on;
  }
}
for (const tab of $$('[role="tab"]')) {
  tab.addEventListener("click", () => show(tab.id.slice(4)));
  tab.addEventListener("keydown", (e) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    const other = $$('[role="tab"]').find((t) => t !== tab);
    show(other.id.slice(4));
    other.focus();
  });
}

// This system and CPU, as far as the browser tells.
async function here() {
  const ua = navigator.userAgentData;
  const platform = ua?.platform || navigator.platform || "";
  const text = navigator.userAgent;
  const system = /Win/i.test(platform + text)
    ? "windows"
    : /Linux|X11/i.test(platform + text) && !/Android/i.test(text)
      ? "linux"
      : null;
  let arch = /aarch64|arm64/i.test(text) ? "arm64" : null;
  if (ua?.getHighEntropyValues) {
    try {
      const v = await ua.getHighEntropyValues(["architecture", "bitness"]);
      arch = v.architecture === "arm" ? "arm64" : v.bitness === "32" ? "x86" : "x64";
    } catch {}
  }
  return { system, arch: arch || "x64" };
}

function fill(rel) {
  const v = rel.tag_name.replace(/^v/, "");
  for (const n of $$("[data-version]")) n.textContent = v;
  $("#release-line").innerHTML = "";
  $("#release-line").append(
    `Sarab ${v}, released ${day(rel.published_at)}. `,
    Object.assign(document.createElement("a"), { href: rel.html_url, textContent: "What changed" }),
  );
  for (const link of $$("a[data-match]")) {
    const asset = rel.assets.find((a) => new RegExp(link.dataset.match).test(a.name));
    if (!asset) continue;
    link.href = asset.browser_download_url;
    link.title = `${asset.name}, ${mb(asset.size)}`;
    const what = link.closest("li").querySelector(".what");
    if (!link.closest(".pair") && !what.querySelector(".size")) {
      what.append(Object.assign(document.createElement("span"), { className: "size", textContent: mb(asset.size) }));
    }
  }
}

function earlier(list) {
  const items = list.slice(1, 5).map((r) => {
    const li = document.createElement("li");
    li.append(
      Object.assign(document.createElement("a"), { href: r.html_url, textContent: r.tag_name.replace(/^v/, "") }),
      Object.assign(document.createElement("span"), { textContent: day(r.published_at) }),
    );
    return li;
  });
  if (items.length) $("#versions").prepend(...items);
}

async function mark() {
  const { system, arch } = await here();
  const asked = location.hash.slice(1);
  const linked = asked === "linux" || asked === "windows";
  show(linked ? asked : system || "windows");
  if (linked) $("#download").scrollIntoView();
  const hero = $("#hero-download");
  if (system === "windows") {
    const row = $(`#panel-windows li[data-arch="${arch}"]`);
    row?.classList.add("mine");
    row?.querySelector(".what").append(
      Object.assign(document.createElement("span"), { className: "match", textContent: "Matches this PC" }),
    );
    const link = row?.querySelector("a");
    if (link) {
      hero.href = link.href;
      hero.textContent = `Download for Windows ${arch === "arm64" ? "ARM64" : arch}`;
    }
  } else if (system === "linux") {
    hero.href = "#download";
    hero.textContent = "Download for Linux";
  }
}

addEventListener("hashchange", () => {
  const asked = location.hash.slice(1);
  if (asked === "linux" || asked === "windows") {
    show(asked);
    $("#download").scrollIntoView();
  }
});

fetch(`https://api.github.com/repos/${REPO}/releases?per_page=12`)
  .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
  .then((all) => {
    const list = all.filter((r) => !r.draft && !r.prerelease);
    if (!list.length) return;
    fill(list[0]);
    earlier(list);
  })
  .catch(() => {})
  .finally(mark);

// The scene rests when it is out of sight, the way Sarab rests when a window covers it.
const scene = $(".scene");
new IntersectionObserver(([e]) => {
  const resting = e.intersectionRatio < 0.35;
  scene.classList.toggle("resting", resting);
  $("#state-text").textContent = resting ? "Resting: the page is scrolled away" : "Playing";
}, { threshold: [0, 0.35, 1] }).observe(scene);
