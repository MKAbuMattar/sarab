#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
compile_error!(
    "Sarab has only the Windows backend so far; Linux and macOS are PLAN.md Phases 2 and 3."
);
