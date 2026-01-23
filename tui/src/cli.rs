use clap::Parser;

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[arg(short, long)]
    pub goal: Option<String>,

    #[arg(short, long)]
    pub max_coins: Option<u64>,

    #[arg(short, long)]
    pub workspace: Option<String>,

    #[arg(long)]
    pub headless: bool,
}

impl CliArgs {
    pub fn is_headless(&self) -> bool {
        self.headless || self.goal.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_goal() {
        let args = CliArgs::try_parse_from(&["test", "--goal", "Build a site"]).unwrap();
        assert_eq!(args.goal, Some("Build a site".to_string()));
        assert!(args.is_headless());
    }

    #[test]
    fn test_parse_coins() {
        let args = CliArgs::try_parse_from(&["test", "--max-coins", "50"]).unwrap();
        assert_eq!(args.max_coins, Some(50));
        assert!(!args.is_headless()); // Goal is missing, so not automatically headless unless flag is set
    }

    #[test]
    fn test_parse_headless_flag() {
        let args = CliArgs::try_parse_from(&["test", "--headless"]).unwrap();
        assert!(args.headless);
        assert!(args.is_headless());
    }

    #[test]
    fn test_parse_no_args() {
        let args = CliArgs::try_parse_from(&["test"]).unwrap();
        assert_eq!(args.goal, None);
        assert!(!args.headless);
        assert!(!args.is_headless());
    }
}
