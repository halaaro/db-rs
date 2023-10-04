use std::{
    io::{BufRead, ErrorKind},
    path::Path,
    process::exit,
};

use clap::{Parser, Subcommand};

pub(crate) enum Source {
    Arg(String),
    File(String),
    FileLine(String),
    StdIn,
    StdInLine,
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum Commands {
    Connect(ArgsConnect),
    Query(ArgsQuery),
    Execute(ArgsExecute),
}

#[derive(clap::Parser, Clone, Debug)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(clap::Args, Clone, Debug)]
pub(crate) struct ArgsConnect {

    #[arg(short, long)]
    pub connection_string: Option<String>,
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(clap::Parser, Clone, Debug)]
pub(crate) struct ArgsQuery {
    #[arg(short, long)]
    pub connection_string: Option<String>,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long)]
    pub query: Option<String>,
    #[arg(short, long)]
    pub format: Option<OutputFormat>,
}

#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub(crate) enum OutputFormat {
    #[default]
    Json,
    Text,
}

#[derive(clap::Parser, Clone, Debug)]
pub(crate) struct ArgsExecute {
    #[arg(short, long)]
    pub connection_string: Option<String>,
    #[arg(short, long)]
    pub name: Option<String>,
    #[arg(short, long)]
    pub script: Option<String>,
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
