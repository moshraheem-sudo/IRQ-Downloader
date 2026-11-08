//! IRQ Downloader release checks and self-installation from its own GitHub releases.

use crate::i18n::{t, tf};
use crate::{default_data_dir, open_window, Ui};
use serde_json::Value;
use slint::ComponentHandle;
use std::path::Path;
use swiftget_engine::selfupdate::{self, Release};

const REPO: &str = "moshraheem-sudo/IRQ-Downloader";
const SETUP_ASSET: &str = "IRQDownloader-Setup.exe";

// Keep the GitHub API operation on the app's Tokio runtime.
async fn latest_async(client: &reqwest::Client) -> anyhow::Result<Option<(Release, String)>> {
    let response = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header(reqwest::header::USER_AGENT, "IRQ-Downloader")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("No IRQ Downloader release has been published yet. Publish a release with the {SETUP_ASSET} asset first.");
    }
    let body = response.error_for_status()?.text().await?;
    let data: Value = serde_json::from_str(&body)?;
    let tag = data["tag_name"].as_str().unwrap_or_default();
    let version = tag.trim_start_matches(['v', 'V']);
    if !selfupdate::is_newer(version, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    let asset = data["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(SETUP_ASSET)))
        .ok_or_else(|| anyhow::anyhow!("Release {tag} has no {SETUP_ASSET} installer asset"))?;
    let url = asset["browser_download_url"].as_str().unwrap_or_default();
    anyhow::ensure!(
        url.starts_with(&format!("https://github.com/{REPO}/releases/download/")),
        "The installer URL is not from the IRQ Downloader repository"
    );
    let digest = asset["digest"].as_str().unwrap_or_default();
    let sha256 = digest.strip_prefix("sha256:").unwrap_or_default();
    anyhow::ensure!(
        sha256.len() == 64 && sha256.chars().all(|c| c.is_ascii_hexdigit()),
        "GitHub did not provide a valid SHA-256 digest for the installer"
    );
    Ok(Some((
        Release {
            version: version.to_string(),
            url: url.to_string(),
            sha256: sha256.to_string(),
            sig: String::new(),
        },
        tag.to_string(),
    )))
}

fn show_check(ui: &Ui, automatic: bool) {
    let Some(dialog) = ui.updater.upgrade() else {
        return;
    };
    if !automatic {
        open_window!(dialog);
    }
    dialog.set_state(0);
    dialog.set_heading(t("Checking IRQ Downloader releases…").into());
    dialog.set_detail(format!("Current version: {}", env!("CARGO_PKG_VERSION")).into());
    dialog.set_progress(-1.0);
    let weak = dialog.as_weak();
    let client = ui.ctx.mgr.client();
    ui.ctx.rt.spawn(async move {
        let result = latest_async(&client).await;
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(d) = weak.upgrade() {
                match result {
                    Ok(Some((release, tag))) => {
                        if automatic {
                            open_window!(d);
                        }
                        d.set_state(2);
                        d.set_heading(
                            format!("IRQ Downloader {} is available", release.version).into(),
                        );
                        d.set_detail(
                            format!(
                                "Current version: {} · Release: {tag}",
                                env!("CARGO_PKG_VERSION")
                            )
                            .into(),
                        );
                        d.set_progress(-1.0);
                    }
                    Ok(None) => {
                        d.set_state(1);
                        d.set_heading(
                            tf(
                                "IRQ Downloader is up to date (version {}).",
                                &[&env!("CARGO_PKG_VERSION")],
                            )
                            .into(),
                        );
                        d.set_detail(t("No newer published release was found.").into());
                        d.set_progress(-1.0);
                    }
                    Err(e) => {
                        if automatic {
                            return;
                        }
                        d.set_state(4);
                        d.set_heading(t("Could not check for IRQ Downloader updates.").into());
                        d.set_detail(format!("{e:#}").into());
                        d.set_progress(-1.0);
                    }
                }
            }
        });
        // Automatic launch check only informs about available releases; download
        // and install starts when the user presses the explicit button.
        let _ = automatic;
    });
}

pub fn spawn(ui: &Ui) {
    if !selfupdate::installed() {
        return;
    }
    let ui = ui.clone();
    slint::Timer::single_shot(std::time::Duration::from_secs(4), move || {
        show_check(&ui, true)
    });
}

pub fn check_now(ui: &Ui) {
    show_check(ui, false);
}

pub fn download_and_install(ui: &Ui) {
    let Some(dialog) = ui.updater.upgrade() else {
        return;
    };
    dialog.set_state(3);
    dialog.set_heading(t("Downloading IRQ Downloader update…").into());
    dialog
        .set_detail(t("The installer will be verified and run when the download finishes.").into());
    dialog.set_progress(-1.0);
    let weak = dialog.as_weak();
    let client = ui.ctx.mgr.client();
    let rt = ui.ctx.rt.clone();
    let dir = default_data_dir().join("updates");
    let weak_progress = weak.clone();
    rt.spawn(async move {
        let result = async {
            let (release, _) = latest_async(&client)
                .await?
                .ok_or_else(|| anyhow::anyhow!("There is no newer published release"))?;
            let path =
                selfupdate::download_with_progress(&client, &release, &dir, &move |done, total| {
                    let progress = total
                        .map(|n| if n > 0 { done as f32 / n as f32 } else { -1.0 })
                        .unwrap_or(-1.0);
                    let w = weak_progress.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(d) = w.upgrade() {
                            d.set_progress(progress);
                        }
                    });
                })
                .await?;
            Ok::<_, anyhow::Error>((path, release.sha256))
        }
        .await;
        match result {
            Ok((path, hash)) => {
                if let Some(d) = weak.upgrade() {
                    d.set_heading(t("Installing IRQ Downloader update…").into());
                    d.set_detail(
                        t("IRQ Downloader will close and reopen after installation.").into(),
                    );
                }
                let install = selfupdate::run_installer(Path::new(&path), &hash, true);
                if let Err(e) = install {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(d) = weak.upgrade() {
                            d.set_state(4);
                            d.set_heading(t("Could not install the update.").into());
                            d.set_detail(format!("{e:#}").into());
                        }
                    });
                } else {
                    std::process::exit(0);
                }
            }
            Err(e) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(d) = weak.upgrade() {
                        d.set_state(4);
                        d.set_heading(t("Could not download the update.").into());
                        d.set_detail(format!("{e:#}").into());
                        d.set_progress(-1.0);
                    }
                });
            }
        }
    });
}
