// SPDX-License-Identifier: MIT

/// Cross-platform command splitter respecting host-platform conventions.
///
/// On Windows, backslashes are file path separators (e.g. `C:\Windows\py.exe`)
/// and must not be mangled as POSIX escape sequences. On Unix, POSIX shell
/// quote and escape semantics apply.

#[must_use]
pub fn split_command(command: &str) -> Option<Vec<String>> {
    #[cfg(windows)]
    {
        split_command_windows(command)
    }
    #[cfg(not(windows))]
    {
        split_command_unix(command)
    }
}

/// POSIX / Unix command line splitting rules
#[must_use]
pub fn split_command_unix(command: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut token_started = false;

    for c in command.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            token_started = true;
        } else if c == '\\' && !in_single_quote {
            escaped = true;
        } else if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            token_started = true;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            token_started = true;
        } else if (c == ' ' || c == '\t' || c == '\n') && !in_single_quote && !in_double_quote {
            if token_started {
                args.push(std::mem::take(&mut current));
                token_started = false;
            }
        } else {
            current.push(c);
            token_started = true;
        }
    }

    if in_single_quote || in_double_quote || escaped {
        return None;
    }
    if token_started {
        args.push(current);
    }
    Some(args)
}

/// Windows `CommandLineToArgvW` argument splitting rules
#[must_use]
pub fn split_command_windows(command: &str) -> Option<Vec<String>> {
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = command.chars().peekable();
    let mut in_quotes = false;
    let mut arg_started = false;

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let mut count = 1usize;
                while chars.peek() == Some(&'\\') {
                    chars.next();
                    count += 1;
                }
                if chars.peek() == Some(&'"') {
                    for _ in 0..(count / 2) {
                        cur.push('\\');
                    }
                    arg_started = true;
                    if count % 2 == 1 {
                        chars.next();
                        cur.push('"');
                    }
                } else {
                    for _ in 0..count {
                        cur.push('\\');
                    }
                    arg_started = true;
                }
            }
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cur.push('"');
                    arg_started = true;
                } else {
                    in_quotes = false;
                }
            }
            '"' => {
                in_quotes = true;
                arg_started = true;
            }
            c if (c == ' ' || c == '\t') && !in_quotes => {
                if arg_started {
                    args.push(std::mem::take(&mut cur));
                    arg_started = false;
                }
            }
            _ => {
                cur.push(c);
                arg_started = true;
            }
        }
    }

    if in_quotes {
        return None;
    }
    if arg_started {
        args.push(cur);
    }
    Some(args)
}
