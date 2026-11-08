//! IRQ Downloader is distributed as a standalone build. Its updater is kept
//! disabled so this build cannot install releases from the upstream Ezoo app.

use crate::i18n::t;
use crate::{open_window, Ctx, Ui};
use slint::ComponentHandle;

pub fn spawn(_ctx: &Ctx) {}

pub fn check_now(ui: &Ui) {
    let Some(dialog) = ui.updater.upgrade() else { return };
    open_window!(dialog);
    dialog.set_state(4);
    dialog.set_heading(t("Automatic updates are not configured for this IRQ Downloader build.").into());
    dialog.set_detail(t("Install a newer IRQ Downloader release when one is published.").into());
    dialog.set_progress(-1.0);
}
