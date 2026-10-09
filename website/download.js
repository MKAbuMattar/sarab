// Fills the download links from the newest GitHub release, and marks the one for this PC.
// Without this script (or without network), the 0.0.9 links in the page still work.
const REPO = "MKAbuMattar/sarab";
const NAMES = { x64: "x64", arm64: "ARM64", x86: "x86" };

async function myArch() {
  const ua = navigator.userAgentData;
  if (ua?.getHighEntropyValues) {
    try {
      const v = await ua.getHighEntropyValues(["architecture", "bitness"]);
      if (v.architecture === "arm") return "arm64";
      if (v.bitness === "32") return "x86";
      return "x64";
    } catch {}
  }
  return /WOW64|Win64|x64/.test(navigator.userAgent) ? "x64" : null;
}

const isWindows = () =>
  (navigator.userAgentData?.platform || navigator.userAgent).includes("Win");
const mb = (bytes) => `${(bytes / 1048576).toFixed(1)} MB`;
const day = (iso) =>
  new Date(iso).toLocaleDateString("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  });

function fill(rel) {
  const v = rel.tag_name.replace(/^v/, "");
  document.querySelector("[data-latest]").textContent = `Version ${v}`;
  document.querySelector("[data-version]").textContent = v;
  document.querySelector("[data-latest-line]").textContent =
    `Sarab ${v}, released ${day(rel.published_at)}.`;
  document.querySelector("#notes").href = rel.html_url;
  for (const card of document.querySelectorAll(".dl[data-arch]")) {
    const asset = rel.assets.find((a) =>
      a.name.endsWith(`_${card.dataset.arch}-setup.exe`),
    );
    if (!asset) {
      card.hidden = true;
      continue;
    }
    const link = card.querySelector(".button");
    link.href = asset.browser_download_url;
    link.dataset.size = mb(asset.size);
    link.querySelector(".size").textContent = mb(asset.size);
    card.querySelector(".file").textContent = asset.name;
  }
}

function earlier(list) {
  document.querySelector("#versions").replaceChildren(
    ...list.slice(1, 5).map((r) => {
      const li = document.createElement("li");
      const a = document.createElement("a");
      a.href = r.html_url;
      a.textContent = r.tag_name.replace(/^v/, "");
      const when = document.createElement("span");
      when.textContent = day(r.published_at);
      li.append(a, when);
      return li;
    }),
  );
}

async function mark() {
  const arch = isWindows() ? await myArch() : null;
  const hero = document.querySelector("#hero-download");
  if (!arch) {
    // Not Windows, or not sure which Windows: show every choice.
    hero.href = "#download";
    return;
  }
  const card = document.querySelector(`.dl[data-arch="${arch}"]`);
  if (!card || card.hidden) return;
  card.classList.add("mine");
  const link = card.querySelector(".button");
  hero.href = link.href;
  document.querySelector("#hero-label").textContent =
    `Download for Windows ${NAMES[arch]}`;
  document.querySelector("#hero-fine").textContent =
    `Windows 10 (1903 or later) and 11 · ${link.dataset.size || link.querySelector(".size").textContent} · No administrator rights`;
}

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
