use anyhow::{Result, bail};

/// Explain a command line in plain English.
///
/// v0: honest stub. The provider-agnostic AI backend (Anthropic / OpenAI / local
/// models via Ollama) is planned for v0.2 — see `docs/ROADMAP.md`. The design is
/// sketched in `docs/ARCHITECTURE.md` so the CLI surface is already stable:
/// `qmark explain "<command line>"` will not change when the backend lands.
pub fn explain(line: &str) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        bail!(r#"nothing to explain — try: qmark explain "tar -xzf archive.tar.gz""#);
    }

    println!("── qmark ── explain {}", "─".repeat(40));
    println!();
    println!("    {line}");
    println!();
    match std::env::var("QMARK_AI_PROVIDER") {
        Ok(provider) => {
            println!(
                "Provider `{provider}` is configured, but the AI backend is not wired up yet."
            );
            println!("It is the next milestone — see docs/ROADMAP.md (v0.2).");
        }
        Err(_) => {
            println!("The AI backend is not wired up yet (this is the scaffold release).");
            println!("Once it lands, set QMARK_AI_PROVIDER and QMARK_AI_API_KEY to enable it.");
            println!("See docs/ROADMAP.md (v0.2) for the plan.");
        }
    }
    Ok(())
}
