use std::io::Write;

use crossterm::{
    cursor, event::{self, Event, KeyCode},
    style::Stylize,
    terminal::{self, ClearType},
    ExecutableCommand,
};

use crate::deck::{markdown_to_text, Deck};

pub fn present(deck: &Deck) -> std::io::Result<()> {
    if deck.slides.is_empty() {
        println!("No slides found.");
        return Ok(());
    }
    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode()?;
    let mut idx = 0usize;
    loop {
        stdout.execute(terminal::Clear(ClearType::All))?;
        stdout.execute(cursor::MoveTo(0, 0))?;
        terminal::disable_raw_mode()?;
        draw_slide(&deck, idx);
        terminal::enable_raw_mode()?;
        if let Event::Key(k) = event::read()? {
            match k.code {
                KeyCode::Right | KeyCode::Down | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('n') | KeyCode::Char('l') => {
                    if idx + 1 < deck.slides.len() {
                        idx += 1;
                    }
                }
                KeyCode::Left | KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('h') => {
                    idx = idx.saturating_sub(1);
                }
                KeyCode::PageDown => {
                    idx = (idx + 5).min(deck.slides.len() - 1);
                }
                KeyCode::PageUp => {
                    idx = idx.saturating_sub(5);
                }
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Home | KeyCode::Char('g') => idx = 0,
                KeyCode::End | KeyCode::Char('G') => idx = deck.slides.len() - 1,
                _ => {}
            }
        }
    }
    terminal::disable_raw_mode()?;
    stdout.execute(terminal::Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    writeln!(stdout, "Done. {} slides.", deck.slides.len())?;
    Ok(())
}

fn draw_slide(deck: &Deck, idx: usize) {
    let slide = &deck.slides[idx];
    let text = markdown_to_text(&slide.source);
    let header = format!(
        "— slide {}/{} —  (←/→ navigate, PgUp/PgDn jump 5, g/G first/last, q quit)",
        idx + 1,
        deck.slides.len()
    );
    println!("{}", header.dark_grey());
    println!();
    println!("{text}");
    if let Some(m) = slide.media.first() {
        let kind = if m.is_video() { "video" } else { "image" };
        let flags = format!(
            "{kind}: {}{}{}{}",
            m.src,
            if m.loop_video { " loop" } else { "" },
            if m.muted { " muted" } else { "" },
            if !m.autoplay { " autoplay=false" } else { "" },
        );
        println!();
        println!("{}", format!("▶ {flags} (plays in HTML/PPTX export)").dark_grey());
    }
}
