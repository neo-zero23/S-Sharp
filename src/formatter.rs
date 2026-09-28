use crate::error::SSharpError;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Canonical formatting for S# source: one statement per line.
///
/// Splits statements at `.` terminators while respecting strings
/// (`"Hello, world."` keeps its period), decimals (`3.14` is untouched),
/// bracket depth and `#` comments (preserved verbatim). Blank lines are
/// collapsed and stray indentation is removed — except inside
/// multi-line strings, where every byte is significant.
///
/// The source is fully parsed first, so `fmt` refuses to reformat broken
/// code instead of mangling it.
pub fn format_source(source: &str) -> Result<String, SSharpError> {
    // Validate first: never reformat a program that does not parse.
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()?;

    let mut out = String::new();
    let mut line = String::new();
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut depth: usize = 0;

    // Pushes the pending line unless it is blank (outside strings).
    // When `force` is set (statement terminator), even whitespace-only
    // content before the '.' is flushed — callers ensure it is not empty.
    let flush_line = |line: &mut String, out: &mut String, in_string: bool| {
        if in_string {
            out.push_str(line);
            out.push('\n');
        } else {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push('\n');
            }
        }
        line.clear();
    };

    while let Some(ch) = chars.next() {
        if in_string {
            line.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    line.push(next);
                }
            } else if ch == '"' {
                in_string = false;
            } else if ch == '\n' {
                // Newline inside a multi-line string: keep the line as-is.
                out.push_str(&line);
                line.clear();
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                line.push(ch);
            }
            '#' => {
                // Comment: copy to end of line verbatim.
                line.push(ch);
                for c in chars.by_ref() {
                    line.push(c);
                    if c == '\n' {
                        break;
                    }
                }
                // The comment owns the rest of its line: emit it now so a
                // statement starting on the next line is not glued to it.
                if line.ends_with('\n') {
                    line.pop();
                }
                flush_line(&mut line, &mut out, false);
            }
            '(' | '[' => {
                depth += 1;
                line.push(ch);
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                line.push(ch);
            }
            '.' => {
                let next_is_digit = chars.peek().is_some_and(|n| n.is_ascii_digit());
                line.push(ch);
                if depth == 0 && !next_is_digit {
                    // Statement terminator: one statement per line.
                    flush_line(&mut line, &mut out, false);
                }
            }
            '\n' | '\r' => {
                flush_line(&mut line, &mut out, false);
            }
            c if c.is_whitespace() => {
                // Skip indentation at line start; keep spacing inside lines.
                if !line.trim().is_empty() {
                    line.push(c);
                }
            }
            _ => line.push(ch),
        }
    }

    // Trailing text without a closing '.' (whitespace only, since the
    // program parsed fine): flush leftovers, if any.
    flush_line(&mut line, &mut out, in_string);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_liner_expands() {
        let out = format_source("when (test). save 1 to x. display x.").unwrap();
        assert_eq!(out, "when (test).\nsave 1 to x.\ndisplay x.\n");
    }

    #[test]
    fn test_strings_with_periods_untouched() {
        let out = format_source("when (test). display \"Hello, world.\". display 3.14.").unwrap();
        assert_eq!(out, "when (test).\ndisplay \"Hello, world.\".\ndisplay 3.14.\n");
    }

    #[test]
    fn test_comments_preserved() {
        let out = format_source("# header comment\nwhen (test). # trailing.\n display x. # done.\n").unwrap();
        assert_eq!(
            out,
            "# header comment\nwhen (test).\n# trailing.\ndisplay x.\n# done.\n"
        );
    }

    #[test]
    fn test_collapses_blanks_and_trims() {
        let out = format_source("when (test).\n\n\n   display x.   \n").unwrap();
        assert_eq!(out, "when (test).\ndisplay x.\n");
    }

    #[test]
    fn test_multiline_statement_kept() {
        let out = format_source("when (test).\nif (a),\ndisplay \"x\".\n").unwrap();
        assert_eq!(out, "when (test).\nif (a),\ndisplay \"x\".\n");
    }

    #[test]
    fn test_broken_code_is_rejected() {
        assert!(format_source("when (test). display .").is_err());
        assert!(format_source("display x.").is_err());
    }
}
