function ask({ title, body, ok, cancel = null, danger = false }) {
  const d = $("#ask"),
    okBtn = $("#ask-ok"),
    cancelBtn = $("#ask-cancel");
  $("#ask-title").textContent = title;
  $("#ask-body").textContent = body;
  okBtn.textContent = ok;
  okBtn.className = danger ? "danger-fill" : "accent";
  cancelBtn.hidden = !cancel;
  cancelBtn.textContent = cancel ?? "";
  d.returnValue = "cancel";
  d.showModal();
  (danger && cancel ? cancelBtn : okBtn).focus();
  return new Promise((resolve) =>
    d.addEventListener("close", () => resolve(d.returnValue === "ok"), {
      once: true,
    }),
  );
}
