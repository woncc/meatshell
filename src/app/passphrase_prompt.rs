//! In-memory export and WebDAV sync passphrase prompt (#26).
//! The passphrase is never written to settings, logs, or status text.

use std::path::PathBuf;

use zeroize::{Zeroize, Zeroizing};

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PromptKind {
    Idle,
    Export,
    Import,
    WebDavUpload,
    WebDavDownload,
}

pub(super) struct PassphraseState {
    pub kind: PromptKind,
    pub sync_pass: Zeroizing<String>,
    pub import_path: Option<PathBuf>,
    pub download: Option<Vec<u8>>,
}

impl PassphraseState {
    pub(super) fn new() -> Self {
        Self {
            kind: PromptKind::Idle,
            sync_pass: Zeroizing::new(String::new()),
            import_path: None,
            download: None,
        }
    }
}

pub(super) fn clear_passphrase_fields(window: &AppWindow) {
    window.set_passphrase_value(SharedString::new());
    window.set_passphrase_confirm(SharedString::new());
}

pub(super) fn show_passphrase_dialog(window: &AppWindow, kind: PromptKind) {
    let (title, message, confirming) = match kind {
        PromptKind::Export => (
            t("导出连接", "Export connections"),
            t(
                "至少 8 个字符。建议使用较长的口令短语，不要复用登录密码。口令不会被保存。",
                "At least 8 characters. A long passphrase is better than a reused login password. It is not saved.",
            ),
            true,
        ),
        PromptKind::Import => (
            t("导入连接", "Import connections"),
            t(
                "此文件已用口令加密。口令错误或文件被篡改时，不会导入任何连接。",
                "This file is encrypted with a passphrase. A wrong passphrase or a damaged file imports nothing.",
            ),
            false,
        ),
        PromptKind::WebDavUpload => (
            t("WebDAV 同步口令", "WebDAV sync passphrase"),
            t(
                "至少 8 个字符。同步口令只留在本次运行的内存中，不会写入设置。",
                "At least 8 characters. The sync passphrase stays in memory for this session and is not saved in settings.",
            ),
            true,
        ),
        PromptKind::WebDavDownload => (
            t("WebDAV 同步口令", "WebDAV sync passphrase"),
            t(
                "输入同步口令以解密远端文件。口令不会写入设置。口令错误时不会导入任何连接。",
                "Enter the sync passphrase to decrypt the remote file. It is not saved. A wrong passphrase imports nothing.",
            ),
            false,
        ),
        PromptKind::Idle => return,
    };
    window.set_passphrase_title(title.into());
    window.set_passphrase_message(message.into());
    window.set_passphrase_confirming(confirming);
    window.set_passphrase_error(SharedString::new());
    clear_passphrase_fields(window);
    window.set_passphrase_open(true);
}

pub(super) fn passphrase_error_text(err: &str) -> String {
    if err == crate::config::ERR_PASSPHRASE_TOO_SHORT {
        t(
            "口令至少需要 8 个字符",
            "passphrase must be at least 8 characters",
        )
        .to_string()
    } else if err == crate::config::ERR_PASSPHRASE_MISMATCH {
        t("两次输入的口令不一致", "the two passphrases do not match").to_string()
    } else if err == crate::config::ERR_PASSPHRASE_TOO_LONG {
        t("口令过长", "passphrase is too long").to_string()
    } else if err == crate::config::ERR_EXPORT_AUTH {
        t(
            "口令不正确或文件已损坏，未导入任何连接",
            "passphrase incorrect or file is damaged; nothing was imported",
        )
        .to_string()
    } else if err == crate::config::ERR_EXPORT_TRUNCATED {
        t("导出文件不完整", "export file is truncated").to_string()
    } else if err == crate::config::ERR_EXPORT_KDF {
        t(
            "导出文件的密钥派生参数不被允许",
            "export file key-derivation parameters are not allowed",
        )
        .to_string()
    } else if err == crate::config::ERR_EXPORT_VERSION {
        t(
            "不支持的导出文件版本",
            "unsupported passphrase export version",
        )
        .to_string()
    } else if err == crate::config::ERR_PASSPHRASE_REQUIRED {
        t(
            "此导出文件需要口令，不能绕过",
            "this export is protected by a passphrase",
        )
        .to_string()
    } else {
        err.to_string()
    }
}

pub(super) fn legacy_reexport_note() -> &'static str {
    t(
        "这是旧格式，不安全，请用口令重新导出",
        "legacy format is not secure; re-export with a passphrase",
    )
}

pub(super) fn zeroize_secret(value: &mut String) {
    value.zeroize();
}
