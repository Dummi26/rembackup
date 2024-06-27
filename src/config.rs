use std::{
    iter::{Enumerate, Peekable},
    path::{Path, PathBuf},
    str::Lines,
};

#[derive(Clone, Copy, Debug)]
pub struct FsEntry<'a> {
    pub path: &'a Path,
    pub is_directory: bool,
}

#[derive(Debug)]
pub struct Ignore(pub Vec<Specifier>);
#[derive(Debug)]
pub enum Specifier {
    /// Everything in this config-part will be explicitly ignored.
    /// Files in ignored directories, even if they would later match,
    /// will not be seen my rembackup.
    Except(Ignore),
    Entries(Match),
    Files(Match),
    /// Descend into a directory.
    /// Contains an inner config, which uses paths relative to the matched directory.
    InDir {
        dir: Match,
        inner: Ignore,
    },
}
#[derive(Debug)]
pub enum Match {
    Any,
    Eq(PathBuf),
    Glob(String),
}

impl Match {
    pub fn matches(&self, path: &Path) -> bool {
        match self {
            Self::Any => true,
            Self::Eq(v) => v.as_path() == path,
            Self::Glob(v) => path
                .to_str()
                .is_some_and(|path| glob_match::glob_match(v, path)),
        }
    }
}

impl Ignore {
    /// If `self.matches(entry)` is `Some(v)`, returns `v`, if not, returns `false`.
    pub fn matches_or_default(&self, entry: &FsEntry) -> bool {
        self.matches(entry).unwrap_or(false)
    }
    pub fn matches(&self, entry: &FsEntry) -> Option<bool> {
        self.0.iter().rev().filter_map(|v| v.matches(entry)).next()
    }
    /// applies each specifier to each element of the `entries()` iterator.
    /// any specifier overrides all earlier ones,
    /// but the first entry that produces true or false will determine the output.
    pub fn matches_of<'a, I: Iterator<Item = &'a FsEntry<'a>>>(
        &self,
        entries: impl Fn() -> I,
    ) -> Option<bool> {
        self.0
            .iter()
            .rev()
            .filter_map(|v| entries().filter_map(|entry| v.matches(entry)).next())
            .next()
    }
}
impl Specifier {
    pub fn matches(&self, entry: &FsEntry) -> Option<bool> {
        match self {
            Self::Except(inner) => inner.matches(entry).map(std::ops::Not::not),
            Self::Entries(path) => path.matches(entry.path).then_some(true),
            Self::Files(path) => (!entry.is_directory && path.matches(entry.path)).then_some(true),
            Self::InDir { dir, inner } => {
                if inner.0.is_empty() {
                    // this has no inner things, so we just check for this directory
                    // if this is a directory and it matches, then return true
                    (entry.is_directory && dir.matches(entry.path)).then_some(true)
                } else {
                    // this has inner things, so, for every matching parent,
                    // get the relative path (by removing the parent), ...
                    let mut path = entry.path;
                    let mut paths = vec![];
                    while let Some(p) = path.parent() {
                        if dir.matches(p) {
                            if let Ok(p) = entry.path.strip_prefix(p) {
                                let mut e = *entry;
                                e.path = p;
                                paths.push(e);
                            } else {
                                eprintln!("[WARN] Parent {p:?} of path {:?} could not be removed... this is probably be a bug.", entry.path);
                            }
                        }
                        path = p;
                    }
                    // ... and then check if any match
                    inner.matches_of(|| paths.iter())
                }
            }
        }
    }
}

impl Ignore {
    pub fn parse(config: &str) -> Result<Self, String> {
        Self::parsei(&mut config.lines().enumerate().peekable(), 0)
    }
    /// min_indent = parent_indent + 1, or 0 if no parent
    fn parsei(lines: &mut Peekable<Enumerate<Lines>>, min_indent: usize) -> Result<Self, String> {
        let mut indent = None;
        let mut specifiers = vec![];
        loop {
            if let Some((line_nr, full_line)) = lines.peek() {
                let line_nr = *line_nr;
                let indent = {
                    let line = full_line.trim_start();
                    // check indentation
                    let line_start_whitespace = &full_line[0..full_line.len() - line.len()];
                    if let Some(c) = line_start_whitespace.chars().find(|c| *c != ' ') {
                        return Err(format!(
                        "Lines must start with any number of spaces, and no other whitespace character, but line {} contained the '{c:?}' character (Unicode {}).",
                        line_nr + 1, c.escape_unicode()));
                    }
                    let line_indent = line_start_whitespace.len();
                    if line_indent < min_indent {
                        // less indentation than before, go up one layer of recursion
                        break;
                    }
                    if let Some(indent) = indent {
                        // check if we indent more/less than on the first line
                        if line_indent != indent {
                            return Err(format!(
                                "Lines in one part of a config may must all have the same indentation! (expected {indent} spaces, but found {line_indent})"
                            ));
                        }
                    } else {
                        // store the first line's indent
                        indent = Some(line_indent);
                    }
                    line_indent
                };
                // -- indentation is ok, this line belongs to us --
                // because we only used `lines.peek()` until now
                let line = lines.next().unwrap().1.trim_start();
                if line.starts_with("#") {
                    // comment, ignore
                } else {
                    let (line_type, args) =
                        line.split_once(char::is_whitespace).unwrap_or((line, ""));
                    specifiers.push(match line_type.to_lowercase().trim() {
                        "except" => Specifier::Except(Ignore::parsei(lines, indent + 1)?),
                        line_type => match (
                            line_type.chars().next().unwrap_or(' '),
                            line_type.chars().skip(1).next().unwrap_or(' '),
                        ) {
                            ('*', m) => Specifier::Entries(Match::parse_m(args, m, line_nr)?),
                            ('+', m) => Specifier::Files(Match::parse_m(args, m, line_nr)?),
                            ('/', m) => Specifier::InDir {
                                dir: Match::parse_m(args, m, line_nr)?,
                                inner: Ignore::parsei(lines, indent + 1)?,
                            },
                            _ => {
                                return Err(format!(
                                "Got '{line}' (Line {}), but expected one of [[*+/][a=*], except]",
                                line_nr + 1
                            ))
                            }
                        },
                    });
                }
            } else {
                break;
            }
        }
        Ok(Self(specifiers))
    }
}
impl Match {
    fn parse_m(text: &str, m: char, line_nr: usize) -> Result<Self, String> {
        Ok(match m {
            'a' => Self::Any,
            '=' => Self::Eq(text.into()),
            '*' => Self::parse_glob(text, line_nr)?,
            _ => {
                return Err(format!(
                    "[Line {}] unknown match-type '{m}', expected one of [a=*]",
                    line_nr + 1
                ))
            }
        })
    }
    fn parse_glob(text: &str, _line_nr: usize) -> Result<Self, String> {
        Ok(Self::Glob(text.to_owned()))
    }
}
