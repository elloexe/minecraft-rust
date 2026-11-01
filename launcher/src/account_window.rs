//! The Microsoft sign-in window for the public launcher id (`account::PUBLIC_LIVE_ID`).
//!
//! That id signs in through login.live.com, which ends on `oauth20_desktop.srf?code=…`. A normal
//! browser is no use there: Microsoft strips the code from that page's address
//! (`?removed=true`). An embedded window sees the redirect before it loads, as every launcher
//! using this id does, and takes the code from it.
//!
//! The window runs in a process of its own (`claudium-launcher --sign-in-window URL`), so its
//! event loop never meets the launcher's GPUI one. It prints `code=…` on its standard output and
//! exits 0 when Microsoft hands over a code, `error=…` and exits 2 when Microsoft refuses, and
//! exits 1 when the player closes it. Windows only: it draws Microsoft's page with WebView2, which
//! Windows 10 and 11 ship. Elsewhere `available` is false and the sign-in falls back to the
//! browser.

/// The argument that starts the window instead of the launcher.
pub const ARG: &str = "--sign-in-window";

/// What the window's output says.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Code(String),
    Refused(String),
    Closed,
}

/// Whether this platform has the window.
pub const fn available() -> bool {
    cfg!(windows)
}

/// What a navigation to `url` means for the sign-in: the code, Microsoft's refusal, or nothing
/// yet (any other page of the sign-in).
pub fn outcome_of(url: &str) -> Option<Outcome> {
    let rest = url.strip_prefix(crate::account::LIVE_REDIRECT)?;
    let query = rest.split_once('?').map_or("", |(_, q)| q);
    let query = query.split('#').next().unwrap_or("");
    let field = |name: &str| {
        query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k == name).then(|| percent_decode(v))
        })
    };
    if let Some(code) = field("code").filter(|c| !c.is_empty()) {
        return Some(Outcome::Code(code));
    }
    field("error").map(|e| Outcome::Refused(field("error_description").unwrap_or(e)))
}

/// `%xx` and `+` decoded, for Microsoft's error descriptions.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => match s.get(i + 1..i + 3).map(|h| u8::from_str_radix(h, 16)) {
                Some(Ok(b)) => {
                    out.push(b);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The window's output, read back by the launcher.
pub fn parse_output(stdout: &str) -> Outcome {
    for line in stdout.lines() {
        if let Some(code) = line.strip_prefix("code=") {
            return Outcome::Code(code.trim().to_string());
        }
        if let Some(e) = line.strip_prefix("error=") {
            return Outcome::Refused(e.trim().to_string());
        }
    }
    Outcome::Closed
}

/// Opens the window on `url` and blocks until the sign-in ends; returns its outcome. Runs the
/// window as a child process of this executable.
pub fn sign_in(url: &str) -> Result<Outcome, String> {
    let mut cmd = crate::process::command(crate::backend::exe_path());
    cmd.arg(ARG)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let out = cmd
        .output()
        .map_err(|e| format!("Could not open the sign-in window: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    match parse_output(&stdout) {
        Outcome::Closed if out.status.code() != Some(1) => Err(format!(
            "The sign-in window failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        outcome => Ok(outcome),
    }
}

/// The window itself: `claudium-launcher --sign-in-window URL`. Never returns.
#[cfg(windows)]
pub fn run(url: &str) -> ! {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::window::WindowBuilder;

    let event_loop = EventLoopBuilder::<Outcome>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = match WindowBuilder::new()
        .with_title("Sign in with Microsoft")
        .with_inner_size(LogicalSize::new(520.0, 720.0))
        .build(&event_loop)
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(3);
        }
    };
    // WebView2 keeps its profile here, not beside the program (which may not be writable).
    let mut context =
        wry::WebContext::new(Some(crate::backend::data_dir().join("sign-in-webview")));
    let done = Arc::new(AtomicBool::new(false));
    let seen = {
        let (proxy, done) = (proxy.clone(), done.clone());
        move |url: &str| {
            if let Some(outcome) = outcome_of(url) {
                if !done.swap(true, Ordering::SeqCst) {
                    let _ = proxy.send_event(outcome);
                }
                return true;
            }
            false
        }
    };
    let on_load = seen.clone();
    let webview = wry::WebViewBuilder::new_with_web_context(&mut context)
        .with_url(url)
        // A fresh session every time, so "Sign in with another account" works.
        .with_incognito(true)
        .with_navigation_handler(move |url| !seen(&url))
        .with_on_page_load_handler(move |_, url| {
            on_load(&url);
        })
        .build(&window);
    let _webview = match webview {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(3);
        }
    };
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(outcome) => {
                let line = match outcome {
                    Outcome::Code(code) => format!("code={code}"),
                    Outcome::Refused(e) => format!("error={}", e.replace('\n', " ")),
                    Outcome::Closed => String::new(),
                };
                let mut out = std::io::stdout();
                let _ = writeln!(out, "{line}");
                let _ = out.flush();
                std::process::exit(if line.starts_with("code=") { 0 } else { 2 });
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => std::process::exit(1),
            _ => {}
        }
    })
}

/// No window on this platform.
#[cfg(not(windows))]
pub fn run(_url: &str) -> ! {
    eprintln!("The sign-in window is only on Windows.");
    std::process::exit(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_code_from_the_redirect() {
        assert_eq!(
            outcome_of("https://login.live.com/oauth20_desktop.srf?code=M.C5_BAY.2.U.abc&lc=1043"),
            Some(Outcome::Code("M.C5_BAY.2.U.abc".into()))
        );
        assert_eq!(
            outcome_of("https://login.live.com/oauth20_desktop.srf?removed=true"),
            None
        );
        assert_eq!(
            outcome_of("https://login.live.com/oauth20_authorize.srf?code=no"),
            None
        );
        assert_eq!(
            outcome_of(
                "https://login.live.com/oauth20_desktop.srf?error=access_denied&error_description=The+user+said+no%2E"
            ),
            Some(Outcome::Refused("The user said no.".into()))
        );
    }

    #[test]
    fn reads_the_windows_output() {
        assert_eq!(parse_output("code=abc\n"), Outcome::Code("abc".into()));
        assert_eq!(
            parse_output("error=denied\n"),
            Outcome::Refused("denied".into())
        );
        assert_eq!(parse_output(""), Outcome::Closed);
    }
}
