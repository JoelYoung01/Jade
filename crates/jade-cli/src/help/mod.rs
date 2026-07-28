mod topics;

/// Print rich help for a topic path such as `["tasks", "update", "status"]`.
/// An empty path shows the root overview.
pub fn print_topic(path: &[String]) -> anyhow::Result<()> {
    let key = path
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let page = topics::lookup(&key).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown help topic '{}'\n\nTry: jade help\n     jade tasks help\n     jade tasks update status help",
            if key.is_empty() { "(root)" } else { &key }
        )
    })?;
    println!("{page}");
    Ok(())
}

/// If `args` (without program name) requests rich help, return the topic path.
///
/// Recognizes:
/// - `help [path…]`
/// - `… help` (trailing help at any depth)
pub fn extract_help_path(args: &[String]) -> Option<Vec<String>> {
    if args.is_empty() {
        return None;
    }

    let tokens = non_flag_tokens(args);
    if tokens.is_empty() {
        return None;
    }

    if tokens[0] == "help" {
        return Some(tokens[1..].to_vec());
    }

    if let Some(idx) = tokens.iter().position(|t| t == "help") {
        return Some(tokens[..idx].to_vec());
    }

    None
}

fn non_flag_tokens(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        // POSIX end-of-options: skip and keep scanning (do not drop later tokens).
        // `just cli -- help` / `cargo run … -- -- help` otherwise hides `help`.
        if arg == "--" {
            continue;
        }
        if arg.starts_with('-') {
            if takes_value(arg) {
                skip_next = true;
            }
            continue;
        }
        out.push(arg.clone());
    }
    out
}

fn takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--db"
            | "--id"
            | "--title"
            | "--description"
            | "-d"
            | "--status"
            | "--due"
            | "--tag"
            | "-t"
            | "--repeat"
            | "--label"
            | "--root"
            | "--format"
            | "--limit"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_help_path() {
        let args = vec![
            "tasks".into(),
            "update".into(),
            "status".into(),
            "help".into(),
        ];
        assert_eq!(
            extract_help_path(&args),
            Some(vec!["tasks".into(), "update".into(), "status".into()])
        );
    }

    #[test]
    fn leading_help_path() {
        let args = vec!["help".into(), "tasks".into(), "add".into()];
        assert_eq!(
            extract_help_path(&args),
            Some(vec!["tasks".into(), "add".into()])
        );
    }

    #[test]
    fn help_ignores_flags() {
        let args = vec![
            "--db".into(),
            "x.db".into(),
            "tasks".into(),
            "list".into(),
            "help".into(),
        ];
        assert_eq!(
            extract_help_path(&args),
            Some(vec!["tasks".into(), "list".into()])
        );
    }

    #[test]
    fn help_after_end_of_options() {
        let args = vec!["--".into(), "help".into()];
        assert_eq!(extract_help_path(&args), Some(vec![]));

        let args = vec!["--".into(), "tasks".into(), "add".into(), "help".into()];
        assert_eq!(
            extract_help_path(&args),
            Some(vec!["tasks".into(), "add".into()])
        );
    }
}
