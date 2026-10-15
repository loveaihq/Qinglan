//! Command line: `qinglan.exe [address] [--replay dir]` and
//! `qinglan.exe --bench <address> [--replay dir] [--width W --height H] [--dpr D]`.

use std::path::PathBuf;

pub const USAGE: &str = "usage: qinglan.exe [address] [--replay <dir>]\n       qinglan.exe --bench <address> [--replay <dir>] [--width W] [--height H] [--dpr D]";

/// Largest bench picture, in device pixels (the same cap `qinglan-cli render` has).
const MAX_BENCH_PIXELS: f64 = 32_000_000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct BenchOptions {
    pub address: String,
    /// The viewport in CSS px; the page area of the window is this times `dpr` device pixels.
    pub width: f32,
    pub height: f32,
    pub dpr: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    pub address: Option<String>,
    pub replay: Option<PathBuf>,
    pub bench: Option<BenchOptions>,
}

pub fn parse(args: &[String]) -> Result<Options, String> {
    let mut o = Options::default();
    let (mut bench, mut width, mut height, mut dpr) = (false, None, None, None);
    let mut it = args.iter();
    let value = |it: &mut std::slice::Iter<'_, String>, name: &str| it.next().cloned().ok_or_else(|| format!("{name} needs a value"));
    let number = |s: String, name: &str| s.parse::<f32>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("{name}: not a number: {s}"));
    while let Some(a) = it.next() {
        match a.as_str() {
            "--bench" => {
                bench = true;
                o.address = Some(value(&mut it, "--bench")?);
            }
            "--replay" => o.replay = Some(PathBuf::from(value(&mut it, "--replay")?)),
            "--width" => width = Some(number(value(&mut it, "--width")?, "--width")?),
            "--height" => height = Some(number(value(&mut it, "--height")?, "--height")?),
            "--dpr" => dpr = Some(number(value(&mut it, "--dpr")?, "--dpr")?),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => {
                if o.address.is_some() {
                    return Err(format!("more than one address: {s}"));
                }
                o.address = Some(s.to_string());
            }
        }
    }
    if bench {
        let (w, h, d) = (width.unwrap_or(1280.0), height.unwrap_or(800.0), dpr.unwrap_or(1.0));
        if !(100.0..=8192.0).contains(&w) || !(100.0..=8192.0).contains(&h) {
            return Err("--width and --height must be between 100 and 8192".into());
        }
        if !(0.25..=8.0).contains(&d) {
            return Err("--dpr must be between 0.25 and 8".into());
        }
        if f64::from(w * d) * f64::from(h * d) > MAX_BENCH_PIXELS {
            return Err("the bench picture would be larger than 32 million pixels".into());
        }
        let address = o.address.take().unwrap_or_default();
        o.bench = Some(BenchOptions { address, width: w, height: h, dpr: d });
    } else if width.is_some() || height.is_some() || dpr.is_some() {
        return Err("--width, --height and --dpr are for --bench".into());
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Options, String> {
        parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn plain_windows() {
        assert_eq!(p(&[]).unwrap(), Options::default());
        assert_eq!(p(&["example.com"]).unwrap().address.as_deref(), Some("example.com"));
        let o = p(&["https://a.b/", "--replay", "rec"]).unwrap();
        assert_eq!(o.replay, Some(PathBuf::from("rec")));
        assert!(o.bench.is_none());
    }

    #[test]
    fn bench_options_and_defaults() {
        let o = p(&["--bench", "http://x/", "--replay", "d", "--width", "1920", "--height", "1080", "--dpr", "1.5"]).unwrap();
        assert_eq!(o.bench, Some(BenchOptions { address: "http://x/".into(), width: 1920.0, height: 1080.0, dpr: 1.5 }));
        assert!(o.address.is_none());
        let d = p(&["--bench", "http://x/"]).unwrap().bench.unwrap();
        assert_eq!((d.width, d.height, d.dpr), (1280.0, 800.0, 1.0));
    }

    #[test]
    fn mistakes_are_reported() {
        assert!(p(&["--bench"]).is_err());
        assert!(p(&["--nope"]).is_err());
        assert!(p(&["a", "b"]).is_err());
        assert!(p(&["--width", "800", "a"]).is_err());
        assert!(p(&["--bench", "a", "--dpr", "0"]).is_err());
        assert!(p(&["--bench", "a", "--width", "x"]).is_err());
        assert!(p(&["--bench", "a", "--width", "8000", "--height", "8000", "--dpr", "2"]).is_err());
    }
}
