let updateLater = false;
let updateAsked = null;

async function askUpdate(u) {
  updateAsked = u.available.version;
  const plain = (u.available.notes || "").replace(/^#+\s*/gm, "").trim();
  const notes = plain
    ? `${t("update.toastBody", { v: u.available.version })}

${plain}`
    : t("update.toastBody", { v: u.available.version });
  const ok = await ask({
    title: t("update.toastTitle", { v: u.available.version }),
    body: notes,
    ok: t("update.now"),
    cancel: t("update.later"),
  });
  if (ok) run(() => invoke("install_update"));
  else {
    updateLater = true;
    render();
  }
}

function renderUpdate() {
  const u = state.update;
  if (
    u.available &&
    u.progress == null &&
    updateAsked !== u.available.version &&
    !$("#ask").open
  )
    askUpdate(u);
  const bar = $("#update-bar");
  bar.hidden = !u.available || (updateLater && u.progress == null);
  if (u.available) {
    $("#update-text").textContent =
      u.progress != null
        ? t("update.downloading", { v: u.available.version, p: u.progress })
        : t("update.available", { v: u.available.version });
    $("#update-actions").replaceChildren(
      ...(u.progress != null
        ? []
        : [
            btn("accent", "\uE896", t("update.now"), () =>
              run(() => invoke("install_update")),
            ),
            u.available.notes
              ? btn("subtle", "\uE8A5", t("update.notes"), () =>
                  ask({
                    title: t("update.notesTitle", { v: u.available.version }),
                    body: u.available.notes,
                    ok: t("dialog.close"),
                  }),
                )
              : "",
            btn("subtle", "\uE711", t("update.later"), () => {
              updateLater = true;
              render();
            }),
          ]),
    );
  }
  $("#update-status").textContent = u.available
    ? t("update.available", { v: u.available.version })
    : u.error
      ? `${t("update.failed")} ${u.error}`
      : u.checked
        ? t("update.latest", { v: state.version })
        : t("update.never");
}
