//! Executing a typed command line, and the history that goes with it.

use crate::run;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

/// Run one command line.
///
/// The line is resolved again here rather than carried in from the search hit:
/// the hit is a rendering of a decision, and a stale one would start something
/// the user did not see. `Err` is the sentence shown to the user — the same
/// wording the Run dialog uses when a name does not resolve.
#[tauri::command]
pub fn run_command_line(
    state: State<'_, Arc<AppState>>,
    raw: String,
    admin: bool,
) -> Result<(), String> {
    let lang = state.settings.read().language.clone();
    let Some(plan) = run::plan(&raw) else {
        return Err(crate::i18n::tr(&lang, "run.not_found").replace("{name}", raw.trim()));
    };
    let Some(target) = plan.target else {
        let name = plan.missing.unwrap_or_else(|| plan.title.clone());
        return Err(crate::i18n::tr(&lang, "run.not_found").replace("{name}", &name));
    };
    if !run::execute(&target, admin) {
        return Err(crate::i18n::tr(&lang, "run.failed").replace("{name}", &plan.title));
    }
    // Only a command that actually started belongs in the history — the
    // dialog keeps the same rule, and a name that never resolved would sit in
    // the list as a permanent dead entry.
    if state.settings.read().run_remember {
        run::remember(&raw);
    }
    Ok(())
}
