use super::*;
use std::io::Read;

fn webdav_url(base: &str, remote_path: &str) -> Result<String> {
    let base = base.trim().trim_end_matches('/');
    if !base.starts_with("http://") && !base.starts_with("https://") {
        anyhow::bail!(
            "{}",
            t(
                "WebDAV 地址必须以 http:// 或 https:// 开头",
                "WebDAV URL must start with http:// or https://"
            )
        );
    }
    if base.starts_with("http://") && webdav_url_uses_port(base, 5006) {
        anyhow::bail!(
            "{}",
            t(
                "飞牛 WebDAV 的 5006 通常是 HTTPS 端口，请改用 https://...:5006；如果要用 HTTP，请改用 5005 端口",
                "FnOS WebDAV port 5006 is usually HTTPS; use https://...:5006, or use port 5005 for HTTP"
            )
        );
    }
    if base.starts_with("https://") && webdav_url_uses_port(base, 5005) {
        anyhow::bail!(
            "{}",
            t(
                "飞牛 WebDAV 的 5005 通常是 HTTP 端口，请改用 http://...:5005；如果要用 HTTPS，请改用 5006 端口",
                "FnOS WebDAV port 5005 is usually HTTP; use http://...:5005, or use port 5006 for HTTPS"
            )
        );
    }
    if base.ends_with(".json") {
        return Ok(base.to_string());
    }
    let remote = remote_path.trim().trim_start_matches('/');
    if (webdav_url_uses_port(base, 5005) || webdav_url_uses_port(base, 5006))
        && !webdav_url_has_path(base)
        && !remote.contains('/')
    {
        anyhow::bail!(
            "{}",
            t(
                "飞牛 WebDAV 需要写入某个共享目录，不能直接写到根路径；请把 WebDAV 地址改成 https://IP:5006/all/，或把远端文件改成 all/meatshell-connections.json",
                "FnOS WebDAV needs a writable shared folder, not the server root; use https://IP:5006/all/ or set the remote file to all/meatshell-connections.json"
            )
        );
    }
    if remote.is_empty() {
        anyhow::bail!("{}", t("远端文件不能为空", "remote file cannot be empty"));
    }
    Ok(format!("{base}/{remote}"))
}

fn webdav_url_uses_port(base: &str, port: u16) -> bool {
    let Some(authority) = base.split("://").nth(1) else {
        return false;
    };
    let host_port = authority.split('/').next().unwrap_or(authority);
    host_port
        .rsplit_once(':')
        .and_then(|(_, p)| p.parse::<u16>().ok())
        == Some(port)
}

fn webdav_url_has_path(base: &str) -> bool {
    let Some(authority) = base.split("://").nth(1) else {
        return false;
    };
    authority
        .split_once('/')
        .is_some_and(|(_, path)| !path.is_empty())
}

fn webdav_auth_header(username: &str, password: &str) -> Option<String> {
    if username.is_empty() && password.is_empty() {
        return None;
    }
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    Some(format!(
        "Basic {}",
        STANDARD.encode(format!("{username}:{password}"))
    ))
}

fn webdav_auth_req(mut req: ureq::Request, auth: Option<&str>) -> ureq::Request {
    if let Some(auth) = auth {
        req = req.set("Authorization", auth);
    }
    req
}

fn webdav_agent(accept_invalid_certs: bool) -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(20));
    if accept_invalid_certs {
        let tls_config = ureq::rustls::ClientConfig::builder_with_provider(
            ureq::rustls::crypto::ring::default_provider().into(),
        )
        .with_protocol_versions(&[&ureq::rustls::version::TLS12, &ureq::rustls::version::TLS13])
        .expect("rustls ring provider supports TLS 1.2 and TLS 1.3")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(WebDavAcceptAnyCertVerifier))
        .with_no_client_auth();
        builder = builder.tls_config(Arc::new(tls_config));
    }
    builder.build()
}

fn webdav_error(e: ureq::Error) -> anyhow::Error {
    if let ureq::Error::Status(status, response) = e {
        let url = response.get_url().to_string();
        let body = response.into_string().unwrap_or_default();
        let body = body.trim();
        let detail = if body.is_empty() {
            String::new()
        } else {
            format!(": {}", body.chars().take(240).collect::<String>())
        };
        if status == 400 {
            return anyhow::anyhow!(
                "{}: {url}: status code 400{detail}",
                t(
                    "请求被 WebDAV 服务拒绝，请检查地址协议/端口是否匹配，以及远端文件所在目录是否已开启 WebDAV 协议访问",
                    "WebDAV rejected the request; check the URL scheme/port and whether the remote folder allows WebDAV access"
                )
            );
        }
        if status == 405 {
            return anyhow::anyhow!(
                "{}: {url}: status code 405{detail}",
                t(
                    "当前 WebDAV 路径不允许上传；飞牛请写入已开启协议访问的共享目录，例如 WebDAV 地址填 https://IP:5006/all/，或远端文件填 all/meatshell-connections.json",
                    "The current WebDAV path does not allow upload; for FnOS, write into a shared folder such as https://IP:5006/all/ or set remote file to all/meatshell-connections.json"
                )
            );
        }
        return anyhow::anyhow!("{url}: status code {status}{detail}");
    }
    let msg = e.to_string();
    if msg.contains("UnknownIssuer") || msg.contains("invalid peer certificate") {
        anyhow::anyhow!(
            "{} ({msg})",
            t(
                "HTTPS 证书不受信任；如果这是可信 NAS/局域网 WebDAV，请在设置里开启“信任自签名/内网证书”",
                "HTTPS certificate is not trusted; enable \"Trust self-signed / intranet certs\" for a trusted NAS/LAN WebDAV"
            )
        )
    } else {
        anyhow::anyhow!("{msg}")
    }
}

fn webdav_parent_dirs(url: &str) -> Vec<String> {
    let Some((scheme, rest)) = url.split_once("://") else {
        return Vec::new();
    };
    let Some((authority, path)) = rest.split_once('/') else {
        return Vec::new();
    };
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() <= 1 {
        return Vec::new();
    }
    let mut dirs = Vec::with_capacity(parts.len() - 1);
    let mut current = format!("{scheme}://{authority}");
    for part in parts.iter().take(parts.len() - 1) {
        current.push('/');
        current.push_str(part);
        current.push('/');
        dirs.push(current.clone());
        current.pop();
    }
    dirs
}

fn webdav_dir_missing_or_no_create_error() -> anyhow::Error {
    anyhow::anyhow!(
        "{}",
        t(
            "文件夹不存在也无权限创建",
            "folder does not exist and cannot be created"
        )
    )
}

fn webdav_dir_exists(agent: &ureq::Agent, url: &str, auth: Option<&str>) -> Result<bool> {
    let req = webdav_auth_req(agent.request("PROPFIND", url).set("Depth", "0"), auth);
    match req.call() {
        Ok(_) => Ok(true),
        Err(ureq::Error::Status(status, _)) if status == 404 || status == 409 => Ok(false),
        Err(ureq::Error::Status(status, _)) if status == 401 || status == 403 || status == 405 => {
            Err(webdav_dir_missing_or_no_create_error())
        }
        Err(e) => Err(webdav_error(e)),
    }
}

fn webdav_create_dir(agent: &ureq::Agent, url: &str, auth: Option<&str>) -> Result<()> {
    let req = webdav_auth_req(agent.request("MKCOL", url), auth);
    match req.call() {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(status, _)) if status == 405 => Ok(()),
        Err(ureq::Error::Status(status, _))
            if status == 401 || status == 403 || status == 404 || status == 409 =>
        {
            Err(webdav_dir_missing_or_no_create_error())
        }
        Err(e) => Err(webdav_error(e)),
    }
}

fn webdav_ensure_parent_dirs(agent: &ureq::Agent, url: &str, auth: Option<&str>) -> Result<()> {
    for dir in webdav_parent_dirs(url) {
        if !webdav_dir_exists(agent, &dir, auth)? {
            webdav_create_dir(agent, &dir, auth)?;
        }
    }
    Ok(())
}

pub(super) fn webdav_put_bytes(
    base_url: &str,
    remote_path: &str,
    username: &str,
    password: &str,
    accept_invalid_certs: bool,
    body: &[u8],
) -> Result<()> {
    let url = webdav_url(base_url, remote_path)?;
    let agent = webdav_agent(accept_invalid_certs);
    let auth = webdav_auth_header(username, password);
    webdav_ensure_parent_dirs(&agent, &url, auth.as_deref())?;
    let req = webdav_auth_req(
        agent
            .put(&url)
            .set("Content-Type", "application/octet-stream"),
        auth.as_deref(),
    );
    req.send_bytes(body).map(|_| ()).map_err(webdav_error)
}

pub(super) fn webdav_get_bytes(
    base_url: &str,
    remote_path: &str,
    username: &str,
    password: &str,
    accept_invalid_certs: bool,
) -> Result<Vec<u8>> {
    let url = webdav_url(base_url, remote_path)?;
    let agent = webdav_agent(accept_invalid_certs);
    let auth = webdav_auth_header(username, password);
    let req = webdav_auth_req(agent.get(&url), auth.as_deref());
    let mut bytes = Vec::new();
    req.call()
        .map_err(webdav_error)?
        .into_reader()
        .take((crate::config::MAX_EXPORT_BYTES as u64) + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    if bytes.len() > crate::config::MAX_EXPORT_BYTES {
        anyhow::bail!("import file exceeds the 16 MiB limit");
    }
    Ok(bytes)
}

/// WebDAV config sync (#185): settings and manual upload.
pub(super) fn wire_webdav_upload(ctx: &WinCtx, gate: &Rc<RefCell<PassphraseState>>) {
    let WinCtx { store, window, .. } = ctx;
    // Manual upload/download of the passphrase-protected session export.
    // It is intentionally not automatic on startup.
    {
        let s = store.borrow();
        window.set_webdav_enabled(s.webdav_enabled());
        window.set_webdav_url(s.webdav_url().into());
        window.set_webdav_username(s.webdav_username().into());
        window.set_webdav_password(s.webdav_password().into());
        window.set_webdav_remote_path(s.webdav_remote_path().into());
        window.set_webdav_accept_invalid_certs(s.webdav_accept_invalid_certs());
        window.set_webdav_status(String::new().into());
    }
    {
        let store = store.clone();
        window.on_save_webdav_settings(
            move |enabled: bool,
                  url: SharedString,
                  username: SharedString,
                  password: SharedString,
                  remote_path: SharedString,
                  accept_invalid_certs: bool| {
                let mut s = store.borrow_mut();
                s.set_webdav_settings(
                    enabled,
                    url.to_string(),
                    username.to_string(),
                    password.to_string(),
                    remote_path.to_string(),
                    accept_invalid_certs,
                );
                let _ = s.save();
            },
        );
    }
    {
        let weak = window.as_weak();
        let store = store.clone();
        let gate = gate.clone();
        window.on_webdav_upload(move || {
            let Some(w) = weak.upgrade() else { return };
            let enabled = w.get_webdav_enabled();
            let url = w.get_webdav_url().to_string();
            let username = w.get_webdav_username().to_string();
            let mut server_password = w.get_webdav_password().to_string();
            let remote_path = w.get_webdav_remote_path().to_string();
            let accept_invalid_certs = w.get_webdav_accept_invalid_certs();
            {
                let mut s = store.borrow_mut();
                s.set_webdav_settings(
                    enabled,
                    url.clone(),
                    username.clone(),
                    server_password.clone(),
                    remote_path.clone(),
                    accept_invalid_certs,
                );
                let _ = s.save();
            }
            if !enabled {
                zeroize_secret(&mut server_password);
                w.set_webdav_status(t("请先启用 WebDAV 同步", "enable WebDAV sync first").into());
                return;
            }
            let sync_pass = gate.borrow().sync_pass.clone();
            if sync_pass.is_empty() {
                zeroize_secret(&mut server_password);
                {
                    let mut state = gate.borrow_mut();
                    state.kind = PromptKind::WebDavUpload;
                    state.import_path = None;
                    state.download = None;
                }
                show_passphrase_dialog(&w, PromptKind::WebDavUpload);
                return;
            }
            let res = store
                .borrow()
                .export_json(&sync_pass)
                .and_then(|(bytes, count)| {
                    webdav_put_bytes(
                        &url,
                        &remote_path,
                        &username,
                        &server_password,
                        accept_invalid_certs,
                        &bytes,
                    )
                    .map(|_| count)
                });
            zeroize_secret(&mut server_password);
            let msg = match res {
                Ok(count) => format!("{} {}", t("已上传连接", "uploaded connections"), count),
                Err(error) => format!(
                    "{}: {}",
                    t("上传失败", "upload failed"),
                    passphrase_error_text(&error.to_string())
                ),
            };
            w.set_webdav_status(msg.into());
        });
    }
}

/// WebDAV config sync (#185): manual download and import.
pub(super) fn wire_webdav_download(
    ctx: &WinCtx,
    sessions_model: &Rc<VecModel<SessionInfo>>,
    gate: &Rc<RefCell<PassphraseState>>,
) {
    let WinCtx {
        store,
        registry,
        window,
        ..
    } = ctx;
    {
        let weak = window.as_weak();
        let store = store.clone();
        let sessions_model = sessions_model.clone();
        let registry = registry.clone();
        let gate = gate.clone();
        window.on_webdav_download(move || {
            let Some(w) = weak.upgrade() else { return };
            let enabled = w.get_webdav_enabled();
            let url = w.get_webdav_url().to_string();
            let username = w.get_webdav_username().to_string();
            let mut server_password = w.get_webdav_password().to_string();
            let remote_path = w.get_webdav_remote_path().to_string();
            let accept_invalid_certs = w.get_webdav_accept_invalid_certs();
            {
                let mut s = store.borrow_mut();
                s.set_webdav_settings(
                    enabled,
                    url.clone(),
                    username.clone(),
                    server_password.clone(),
                    remote_path.clone(),
                    accept_invalid_certs,
                );
                let _ = s.save();
            }
            if !enabled {
                zeroize_secret(&mut server_password);
                w.set_webdav_status(t("请先启用 WebDAV 同步", "enable WebDAV sync first").into());
                return;
            }
            let fetched = webdav_get_bytes(
                &url,
                &remote_path,
                &username,
                &server_password,
                accept_invalid_certs,
            );
            zeroize_secret(&mut server_password);
            let bytes = match fetched {
                Ok(bytes) => bytes,
                Err(error) => {
                    w.set_webdav_status(
                        format!(
                            "{}: {}",
                            t("下载失败", "download failed"),
                            passphrase_error_text(&error.to_string())
                        )
                        .into(),
                    );
                    return;
                }
            };
            let kind = ConfigStore::inspect_import_bytes(&bytes);
            if kind == crate::config::ImportKind::Passphrase {
                let sync_pass = gate.borrow().sync_pass.clone();
                if sync_pass.is_empty() {
                    let mut state = gate.borrow_mut();
                    state.kind = PromptKind::WebDavDownload;
                    state.download = Some(bytes);
                    state.import_path = None;
                    drop(state);
                    show_passphrase_dialog(&w, PromptKind::WebDavDownload);
                    return;
                }
                match store
                    .borrow_mut()
                    .import_portable_bytes(&bytes, Some(&sync_pass), false)
                {
                    Ok((summary, _)) => {
                        sync_sessions_for_window(&weak, &store.borrow(), &sessions_model);
                        registry.broadcast_config_changed();
                        w.set_webdav_status(
                            format!(
                                "{} {}, {} {}",
                                t("已导入", "imported"),
                                summary.added,
                                t("跳过", "skipped"),
                                summary.skipped
                            )
                            .into(),
                        );
                    }
                    Err(error) => {
                        let message = error.to_string();
                        if message == crate::config::ERR_EXPORT_AUTH {
                            gate.borrow_mut().sync_pass = zeroize::Zeroizing::new(String::new());
                            let mut state = gate.borrow_mut();
                            state.kind = PromptKind::WebDavDownload;
                            state.download = Some(bytes);
                            drop(state);
                            show_passphrase_dialog(&w, PromptKind::WebDavDownload);
                            if let Some(w) = weak.upgrade() {
                                w.set_passphrase_error(passphrase_error_text(&message).into());
                            }
                        } else {
                            w.set_webdav_status(
                                format!(
                                    "{}: {}",
                                    t("下载失败", "download failed"),
                                    passphrase_error_text(&message)
                                )
                                .into(),
                            );
                        }
                    }
                }
                return;
            }
            match store
                .borrow_mut()
                .import_portable_bytes(&bytes, None, false)
            {
                Ok((summary, legacy)) => {
                    sync_sessions_for_window(&weak, &store.borrow(), &sessions_model);
                    registry.broadcast_config_changed();
                    let mut hint = format!(
                        "{} {}, {} {}",
                        t("已导入", "imported"),
                        summary.added,
                        t("跳过", "skipped"),
                        summary.skipped
                    );
                    if legacy {
                        hint.push_str(" · ");
                        hint.push_str(legacy_reexport_note());
                    }
                    w.set_webdav_status(hint.into());
                }
                Err(error) => w.set_webdav_status(
                    format!(
                        "{}: {}",
                        t("下载失败", "download failed"),
                        passphrase_error_text(&error.to_string())
                    )
                    .into(),
                ),
            }
        });
    }
}
