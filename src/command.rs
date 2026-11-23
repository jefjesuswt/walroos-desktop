//! A command typed on one line, as a shell would split it: spaces separate arguments, "..." and
//! '...' keep spaces, \ escapes the next character. No variables or globs: the container gets
//! the arguments as they are. Same rules as the web panel's command.ts.

pub fn split(line: &str) -> Vec<String> {
    let (mut args, mut cur, mut in_arg, mut quote) = (Vec::new(), String::new(), false, None);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) if c == q => quote = None,
            Some('"') if c == '\\' && chars.peek().is_some() => cur.extend(chars.next()),
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => (quote, in_arg) = (Some(c), true),
            None if c == '\\' && chars.peek().is_some() => {
                cur.extend(chars.next());
                in_arg = true;
            }
            None if c.is_whitespace() => {
                if in_arg {
                    args.push(std::mem::take(&mut cur));
                    in_arg = false;
                }
            }
            None => {
                cur.push(c);
                in_arg = true;
            }
        }
    }
    if in_arg {
        args.push(cur);
    }
    args
}

/// The arguments back on one line, quoting what split would split.
pub fn join(args: &[String]) -> String {
    let plain = |a: &str| !a.is_empty() && a.chars().all(|c| c.is_alphanumeric() || "_@%+=:,./-".contains(c));
    args.iter()
        .map(|a| match plain(a) {
            true => a.clone(),
            false => format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\"")),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_like_a_shell() {
        assert_eq!(split(r#"sh -c "npm run worker""#), ["sh", "-c", "npm run worker"]);
        assert_eq!(split(r"echo it\'s 'a b'  c"), ["echo", "it's", "a b", "c"]);
        assert_eq!(split(r#"say "" x"#), ["say", "", "x"]);
        assert!(split("   ").is_empty());
    }

    #[test]
    fn joins_back_what_it_splits() {
        let args = split(r#"sh -c "echo \"hi\" there" ./run.sh"#);
        assert_eq!(split(&join(&args)), args);
        assert_eq!(join(&args), r#"sh -c "echo \"hi\" there" ./run.sh"#);
    }
}
