function showPage(name) {
  document.querySelectorAll(".page").forEach((p) => {
    p.hidden = p.id !== `page-${name}`;
  });
  document.querySelectorAll(".nav-item[data-page]").forEach((b) => {
    if (b.dataset.page === name) b.setAttribute("aria-current", "page");
    else b.removeAttribute("aria-current");
  });
  try {
    localStorage.setItem("page", name);
  } catch {}
}

function showError(e) {
  $("#error").hidden = !e;
  $("#error-text").textContent = e ? `${t("error.prefix")}: ${e}` : "";
}

async function run(fn) {
  try {
    showError(null);
    await fn();
  } catch (e) {
    showError(String(e));
  }
  await refresh();
}

async function refresh() {
  state = await invoke("state");
  render();
}
