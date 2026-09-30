mod agent;
mod backup;
mod deck;
mod export;
mod pptx;
mod present;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "keynote", version, about = "Markdown-powered slides in your terminal")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold an example deck.md
    Init {
        /// Output path (default: deck.md)
        #[arg(default_value = "deck.md")]
        path: PathBuf,
        /// Overwrite if exists
        #[arg(long)]
        force: bool,
    },
    /// Scaffold a new deck with title and theme
    New {
        /// Output Markdown path
        path: PathBuf,
        /// Deck title
        #[arg(long, default_value = "My Talk")]
        title: String,
        /// Theme name
        #[arg(long, default_value = "dark")]
        theme: String,
        /// Overwrite if exists
        #[arg(long)]
        force: bool,
    },
    /// Parse a deck and list slides
    Parse {
        file: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate a deck: every problem with slide and line
    Check {
        file: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Outline slides: number, lines, headline, media
    Slides {
        file: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Render slides to PNG via headless Chromium plus slides.json
    Render {
        file: PathBuf,
        /// 1-indexed slide to render (omit for all slides)
        #[arg(long)]
        slide: Option<usize>,
        /// Output PNG file (with --slide) or directory (without)
        #[arg(short, long)]
        output: PathBuf,
        /// Render width (height = width*9/16, default 3840)
        #[arg(long, default_value_t = 3840)]
        width: u32,
        /// Override theme
        #[arg(long)]
        theme: Option<String>,
    },
    /// Export a deck to HTML (animated), PDF (static) or PPTX (slides+video)
    Export {
        file: PathBuf,
        /// Output path (.html, .pdf or .pptx)
        #[arg(short, long, default_value = "deck.html")]
        output: PathBuf,
        /// Override deck title
        #[arg(long)]
        title: Option<String>,
        /// Override theme
        #[arg(long)]
        theme: Option<String>,
        /// PPTX render width (default 1920)
        #[arg(long, default_value_t = 1920)]
        width: u32,
    },
    /// List bundled themes
    Themes {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show backup history for a deck (.keynote-backups beside the file)
    History {
        file: PathBuf,
    },
    /// Restore a deck from a .bak backup
    Restore {
        file: PathBuf,
        /// List backups without restoring
        #[arg(long)]
        list: bool,
        /// Backup file to restore (defaults to newest)
        #[arg(long)]
        backup: Option<PathBuf>,
    },
    /// Print the agent-readable slide format
    Format,
    /// Print or install the agent skill
    Skill {
        /// Set to `install` to install to ~/.agents/skills/keynote/
        target: Option<String>,
    },
    /// Export HTML, try to open it in a browser, then present in terminal
    Open { file: PathBuf },
    /// Present a deck in the terminal (←/→, q to quit)
    Present { file: PathBuf },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("keynote: {e}");
        std::process::exit(1);
    }
}

fn base_dir_of(file: &Path) -> PathBuf {
    file.parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init { path, force } => {
            if path.exists() && !force {
                return Err(format!("{} exists (use --force)", path.display()));
            }
            std::fs::write(&path, export::EXAMPLE_DECK)
                .map_err(|e| format!("write {}: {e}", path.display()))?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Cmd::New { path, title, theme, force } => {
            if path.exists() {
                if !force {
                    return Err(format!("{} exists (use --force)", path.display()));
                }
                backup::backup_file(&path);
            }
            if !deck::BUNDLED_THEMES.contains(&theme.as_str()) {
                return Err(format!(
                    "unknown theme '{theme}' (try: {})",
                    deck::BUNDLED_THEMES.join(", ")
                ));
            }
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
                }
            }
            let body = format!(
                "---\ntitle: {title}\nauthor: You\ntheme: {theme}\nfont: system-ui\n---\n\n# {title}\n\nPress → to continue\n\n---\n\n## Agenda\n\n- Why markdown slides?\n- Demo\n- Q&A\n\n---\n\n# Thank you!\n\nQuestions?\n"
            );
            std::fs::write(&path, body)
                .map_err(|e| format!("write {}: {e}", path.display()))?;
            let base = base_dir_of(&path);
            let _ = std::fs::create_dir_all(base.join("images"));
            let _ = std::fs::create_dir_all(base.join("videos"));
            println!("wrote {}", path.display());
            Ok(())
        }
        Cmd::Parse { file, json } => {
            let deck = deck::Deck::from_file(&file)?;
            if json {
                let v = serde_json::json!({
                    "title": deck.frontmatter.title,
                    "author": deck.frontmatter.author,
                    "theme": deck.frontmatter.theme,
                    "font": deck.frontmatter.font,
                    "slides": deck.slides,
                });
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
            } else {
                println!("{} slide(s)", deck.slides.len());
                for s in &deck.slides {
                    let t = s.title.clone().unwrap_or_else(|| "(untitled)".into());
                    println!("  {}. {}", s.index + 1, t);
                }
            }
            Ok(())
        }
        Cmd::Check { file, json } => {
            let deck = deck::Deck::from_file(&file)?;
            let base = base_dir_of(&file);
            let diags = deck::check_deck(&deck, &base);
            if json {
                println!("{}", serde_json::to_string_pretty(&diags).unwrap());
            } else {
                if diags.is_empty() {
                    println!("ok: {} slide(s), no problems", deck.slides.len());
                } else {
                    for d in &diags {
                        println!(
                            "{} {} slide {} line {}: {}",
                            d.severity, d.code, d.slide, d.line, d.message
                        );
                    }
                }
            }
            if diags.iter().any(|d| d.severity == "error") {
                return Err(format!(
                    "{} error(s) found",
                    diags.iter().filter(|d| d.severity == "error").count()
                ));
            }
            Ok(())
        }
        Cmd::Slides { file, json } => {
            let deck = deck::Deck::from_file(&file)?;
            let out = deck::outline(&deck);
            if json {
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            } else {
                for s in &out {
                    let h = s.headline.clone().unwrap_or_else(|| "(untitled)".into());
                    let m = s.media.clone().unwrap_or_else(|| "-".into());
                    println!(
                        "{}. [lines {}-{}] {} | media: {}",
                        s.number, s.start_line, s.end_line, h, m
                    );
                }
            }
            Ok(())
        }
        Cmd::Render { file, slide, output, width, theme } => {
            let deck = deck::Deck::from_file(&file)?;
            let base = base_dir_of(&file);
            let diags = deck::check_deck(&deck, &base);
            // Group errors per slide for banners; render still writes PNG.
            let mut errors_by_slide: std::collections::HashMap<usize, Vec<String>> =
                std::collections::HashMap::new();
            for d in diags.iter().filter(|d| d.severity == "error" && d.slide > 0) {
                errors_by_slide
                    .entry(d.slide)
                    .or_default()
                    .push(format!("{}: {}", d.code, d.message));
            }
            if let Some(n) = slide {
                if n == 0 || n > deck.slides.len() {
                    return Err(format!("no slide {n} (deck has {} slides)", deck.slides.len()));
                }
                let banner = errors_by_slide.get(&n).map(|v| v.join("; "));
                let html = export::slide_html(
                    &deck,
                    &base,
                    n - 1,
                    banner.as_deref(),
                    theme.clone(),
                );
                let tmp = std::env::temp_dir().join(format!("keynote-slide-{n}.html"));
                std::fs::write(&tmp, html).map_err(|e| format!("write tmp: {e}"))?;
                export::screenshot_png(&tmp, &output, width)?;
                println!("rendered slide {n} → {}", output.display());
                Ok(())
            } else {
                std::fs::create_dir_all(&output)
                    .map_err(|e| format!("mkdir {}: {e}", output.display()))?;
                for s in &deck.slides {
                    let n = s.index + 1;
                    let banner = errors_by_slide.get(&n).map(|v| v.join("; "));
                    let html = export::slide_html(&deck, &base, s.index, banner.as_deref(), theme.clone());
                    let tmp =
                        std::env::temp_dir().join(format!("keynote-slide-{n}.html"));
                    std::fs::write(&tmp, html).map_err(|e| format!("write tmp: {e}"))?;
                    let png = output.join(format!("slide-{n:03}.png"));
                    export::screenshot_png(&tmp, &png, width)?;
                }
                let outline = deck::outline(&deck);
                let json_path = output.join("slides.json");
                std::fs::write(
                    &json_path,
                    serde_json::to_string_pretty(&outline).unwrap(),
                )
                .map_err(|e| format!("write {}: {e}", json_path.display()))?;
                println!(
                    "rendered {} slides → {}/ + slides.json",
                    deck.slides.len(),
                    output.display()
                );
                Ok(())
            }
        }
        Cmd::Export { file, output, title, theme, width } => {
            let deck = deck::Deck::from_file(&file)?;
            let base = base_dir_of(&file);
            let diags = deck::check_deck(&deck, &base);
            let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
            if !errors.is_empty() {
                for d in &errors {
                    eprintln!("error {} slide {} line {}: {}", d.code, d.slide, d.line, d.message);
                }
                return Err(format!("export blocked: {} error(s); finish code fences and fix media first", errors.len()));
            }
            for d in diags.iter().filter(|d| d.severity == "warning") {
                eprintln!("warning {} slide {}: {}", d.code, d.slide, d.message);
            }
            let ext = output.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if ext == "pdf" {
                let html = export::export_html(&deck, &base, title, theme);
                let tmp_html = std::env::temp_dir().join("keynote-export.html");
                let tmp_pdf = std::env::temp_dir().join("keynote-export.pdf");
                std::fs::write(&tmp_html, html).map_err(|e| format!("write tmp: {e}"))?;
                export::print_pdf(&tmp_html, &tmp_pdf)?;
                // Never clobber on failure: we only reach here on success.
                std::fs::copy(&tmp_pdf, &output)
                    .map_err(|e| format!("write {}: {e}", output.display()))?;
            } else if ext == "pptx" {
                let bytes = pptx::export_pptx(&deck, &base, width)?;
                let tmp = output.with_extension("tmp.pptx");
                std::fs::write(&tmp, &bytes).map_err(|e| format!("write {}: {e}", tmp.display()))?;
                std::fs::rename(&tmp, &output).map_err(|e| format!("write {}: {e}", output.display()))?;
            } else {
                let html = export::export_html(&deck, &base, title, theme);
                let tmp = output.with_extension("tmp.html");
                std::fs::write(&tmp, &html).map_err(|e| format!("write {}: {e}", tmp.display()))?;
                std::fs::rename(&tmp, &output).map_err(|e| format!("write {}: {e}", output.display()))?;
            }
            backup::backup_file(&file);
            println!("exported {} slides → {}", deck.slides.len(), output.display());
            Ok(())
        }
        Cmd::Themes { json } => {
            if json {
                let v: Vec<_> = deck::BUNDLED_THEMES.iter().map(|s| s.to_string()).collect();
                println!("{}", serde_json::to_string_pretty(&v).unwrap());
            } else {
                for t in deck::BUNDLED_THEMES {
                    println!("{t}");
                }
            }
            Ok(())
        }
        Cmd::History { file } => {
            let backs = backup::list_backups(&file);
            if backs.is_empty() {
                println!("no backups for {}", file.display());
            } else {
                for b in backs {
                    println!("{}", b.display());
                }
            }
            Ok(())
        }
        Cmd::Restore { file, list, backup } => {
            let backs = backup::list_backups(&file);
            if list || backup.is_none() && backs.is_empty() {
                if backs.is_empty() {
                    println!("no backups for {}", file.display());
                } else {
                    for b in &backs {
                        println!("{}", b.display());
                    }
                    if !list {
                        eprintln!("pass --backup <file> to restore");
                    }
                }
                return Ok(());
            }
            let src = backup.or_else(|| backs.last().cloned()).ok_or("no backups found")?;
            let content =
                std::fs::read(&src).map_err(|e| format!("read {}: {e}", src.display()))?;
            backup::backup_file(&file);
            std::fs::write(&file, content).map_err(|e| format!("write {}: {e}", file.display()))?;
            println!("restored {} ← {}", file.display(), src.display());
            Ok(())
        }
        Cmd::Format => {
            println!("{}", agent::FORMAT_DOC);
            Ok(())
        }
        Cmd::Skill { target } => {
            if target.as_deref() == Some("install") {
                let dest = agent::install_skill()?;
                println!("installed {}", dest.display());
            } else {
                println!("{}", agent::SKILL_TEXT);
            }
            Ok(())
        }
        Cmd::Open { file } => {
            let deck = deck::Deck::from_file(&file)?;
            let base = base_dir_of(&file);
            let html = export::export_html(&deck, &base, None, None);
            let tmp = std::env::temp_dir().join("keynote-open.html");
            std::fs::write(&tmp, html).map_err(|e| format!("write tmp: {e}"))?;
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&tmp).spawn();
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&tmp).spawn();
            println!("opened {} (terminal present next)", tmp.display());
            present::present(&deck).map_err(|e| e.to_string())
        }
        Cmd::Present { file } => {
            let deck = deck::Deck::from_file(&file)?;
            present::present(&deck).map_err(|e| e.to_string())
        }
    }
}
