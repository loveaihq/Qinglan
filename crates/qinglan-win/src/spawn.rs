//! Starting another copy of the program: the command line is built here (safe code), the call is in `sys::process`.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

const QUOTE: u16 = b'"' as u16;
const BACKSLASH: u16 = b'\\' as u16;

/// Append `arg` to `cmd` quoted so that the C runtime (and `CommandLineToArgvW`) parses it back to exactly `arg`:
/// an argument that is empty or holds a space, tab or quote goes in quotes; a quote is preceded by a backslash, and
/// the backslashes before a quote (or before the closing quote) are doubled.
fn push_arg(cmd: &mut Vec<u16>, arg: &[u16]) {
    let quoted = arg.is_empty() || arg.iter().any(|&c| c == QUOTE || c == u16::from(b' ') || c == u16::from(b'\t'));
    if quoted {
        cmd.push(QUOTE);
    }
    let mut backslashes = 0usize;
    for &c in arg {
        if c == BACKSLASH {
            backslashes += 1;
            continue;
        }
        let n = if c == QUOTE { backslashes * 2 + 1 } else { backslashes };
        cmd.extend(std::iter::repeat_n(BACKSLASH, n));
        backslashes = 0;
        cmd.push(c);
    }
    cmd.extend(std::iter::repeat_n(BACKSLASH, if quoted { backslashes * 2 } else { backslashes }));
    if quoted {
        cmd.push(QUOTE);
    }
}

/// The command line for `exe` and `args`, or `None` if any of them holds a NUL (it cannot be passed).
fn command_line(exe: &Path, args: &[&OsStr]) -> Option<Vec<u16>> {
    let mut cmd = Vec::new();
    // The program name is always quoted (a path cannot contain a quote, so no escaping inside).
    let name: Vec<u16> = exe.as_os_str().encode_wide().collect();
    cmd.push(QUOTE);
    cmd.extend_from_slice(&name);
    cmd.push(QUOTE);
    let mut nul = name.contains(&0);
    for a in args {
        let w: Vec<u16> = a.encode_wide().collect();
        nul |= w.contains(&0);
        cmd.push(u16::from(b' '));
        push_arg(&mut cmd, &w);
    }
    (!nul).then_some(cmd)
}

/// Start `exe` with `args` as a new process and do not wait for it. Returns whether it started.
pub fn spawn_process(exe: &Path, args: &[&OsStr]) -> bool {
    command_line(exe, args).is_some_and(|mut cmd| crate::sys::process::create(&mut cmd))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(args: &[&str]) -> String {
        let a: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
        String::from_utf16(&command_line(Path::new(r"C:\Program Files\q\qinglan.exe"), &a).unwrap()).unwrap()
    }

    #[test]
    fn quoting_matches_the_c_runtime() {
        assert_eq!(line(&[]), r#""C:\Program Files\q\qinglan.exe""#);
        assert_eq!(line(&["https://a.test/x?y=1&z=2"]), r#""C:\Program Files\q\qinglan.exe" https://a.test/x?y=1&z=2"#);
        assert_eq!(line(&["a b", ""]), r#""C:\Program Files\q\qinglan.exe" "a b" """#);
        assert_eq!(line(&[r#"say "hi""#]), r#""C:\Program Files\q\qinglan.exe" "say \"hi\"""#);
        // Backslashes are doubled only before a quote, and before the closing quote.
        assert_eq!(line(&[r"C:\dir with space\"]), r#""C:\Program Files\q\qinglan.exe" "C:\dir with space\\""#);
        assert_eq!(line(&[r"C:\plain\dir\"]), r#""C:\Program Files\q\qinglan.exe" C:\plain\dir\"#);
        assert_eq!(line(&[r#"a\"b"#]), r#""C:\Program Files\q\qinglan.exe" "a\\\"b""#);
    }

    #[test]
    fn a_real_process_starts_and_a_missing_one_does_not() {
        let cmd = Path::new(r"C:\Windows\System32\cmd.exe");
        assert!(spawn_process(cmd, &[OsStr::new("/c"), OsStr::new("exit 0")]));
        assert!(!spawn_process(Path::new(r"C:\no such dir\none.exe"), &[]));
    }

    #[test]
    fn a_nul_cannot_be_passed() {
        assert!(command_line(Path::new("q.exe"), &[OsStr::new("a\0b")]).is_none());
    }
}
