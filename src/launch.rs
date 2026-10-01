//! How `keynote` decides between the graphical editor and a CLI command.
use std::path::PathBuf;

const COMMANDS: &[&str] = &[
    "init", "new", "parse", "check", "slides", "render", "export", "themes", "history",
    "restore", "format", "skill", "open", "present", "view", "install", "help",
];

#[derive(Debug, PartialEq, Eq)]
pub enum GuiRequest {
    Start,
    File(PathBuf),
}

/// `None` means clap should parse the arguments (a real subcommand, --help, or --version).
pub fn gui_request(args: &[String]) -> Option<GuiRequest> {
    if args.is_empty() {
        return Some(GuiRequest::Start);
    }
    if args.len() == 1 {
        let arg = args[0].as_str();
        if arg.starts_with('-') || COMMANDS.contains(&arg) {
            return None;
        }
        return Some(GuiRequest::File(PathBuf::from(arg)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_invocation_opens_the_editor() {
        assert_eq!(gui_request(&[]), Some(GuiRequest::Start));
    }

    #[test]
    fn a_deck_path_opens_that_file() {
        let args = vec!["talk.md".into()];
        assert_eq!(
            gui_request(&args),
            Some(GuiRequest::File(PathBuf::from("talk.md")))
        );
        let spaced = vec!["My Talk.md".into()];
        assert_eq!(
            gui_request(&spaced),
            Some(GuiRequest::File(PathBuf::from("My Talk.md")))
        );
    }

    #[test]
    fn subcommands_and_help_stay_with_clap() {
        assert!(gui_request(&["check".into(), "talk.md".into()]).is_none());
        assert!(gui_request(&["--help".into()]).is_none());
        assert!(gui_request(&["--version".into()]).is_none());
        assert!(gui_request(&["view".into(), "talk.md".into()]).is_none());
    }
}
