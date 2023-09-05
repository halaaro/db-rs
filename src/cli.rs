use std::{
    io::{BufRead, ErrorKind},
    path::Path,
    process::exit,
};

pub(crate) enum Source {
    Arg(String),
    File(String),
    FileLine(String),
    StdIn,
    StdInLine,
}

impl Source {
    pub(crate) fn new_any_line(value: String) -> Self {
        match value.as_str() {
            "-" => Source::StdInLine,
            f if Path::new(&f).metadata().is_ok() => Source::FileLine(value),
            _ => Source::Arg(value),
        }
    }

    pub(crate) fn new_any_multiline(value: String) -> Self {
        match value.as_str() {
            "-" => Source::StdIn,
            f if Path::new(&f).metadata().is_ok() => Source::File(value),
            _ => Source::Arg(value),
        }
    }

    pub(crate) fn into_string(self) -> std::io::Result<String> {
        match self {
            Source::Arg(arg) => Ok(arg),
            Source::File(path) => std::fs::read_to_string(path),
            Source::FileLine(path) => Ok(std::fs::read_to_string(path)?
                .lines()
                .next()
                .ok_or_else(|| std::io::Error::from(ErrorKind::UnexpectedEof))?
                .to_string()),
            Source::StdIn => {
                let mut stdin = std::io::stdin().lock();
                let mut buf = String::new();
                let mut line = String::new();
                loop {
                    let cnt = stdin.read_line(&mut line)?;
                    if line.starts_with("--db-break--") || cnt == 0 {
                        break;
                    }
                    buf += &line;
                    line.clear();
                }
                Ok(buf)
            }
            Source::StdInLine => {
                let mut stdin = std::io::stdin().lock();
                let mut buf = String::new();
                stdin.read_line(&mut buf)?;
                Ok(buf)
            }
        }
    }
}

pub(crate) trait ArgGet {
    fn get(&self, name: &'static str) -> Option<String>;

    fn get_required(&self, name: &'static str) -> String {
        match self.get(name) {
            Some(s) => s,
            None => {
                eprintln!("Could not find required argument {name}");
                exit(1);
            }
        }
    }
}

impl ArgGet for Vec<String> {
    fn get(&self, name: &'static str) -> Option<String> {
        let mut iter = self.iter();
        iter.find(|&a| a == name).and(iter.next()).map(String::from)
    }
}

