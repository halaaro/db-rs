#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::panic)]
#![warn(clippy::todo)]
#![allow(unused)]

mod cli;
mod fmt_util;
mod mssql;

use std::{
    io::{self, BufWriter, Write},
    process::exit,
};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    let args = cli::Cli::parse();
    match args.command {
        cli::Commands::Connect(args) => connect(args).await,
        cli::Commands::Query(args) => query(args).await,
        cli::Commands::Execute(args) => execute(args).await,
    }
}

async fn execute(args: cli::ArgsExecute) -> anyhow::Result<()> {
    dbg!(args);
    Ok(())
}

async fn query(mut args: cli::ArgsQuery) -> anyhow::Result<()> {
    // TODO: handle background connections (-d)
    let conn_string =
        { cli::Source::new_any_line(args.connection_string.unwrap_or_default()).into_string()? };
    let mut conn = mssql::Connection::from_string(&conn_string).await?;

    let query_string = cli::Source::new_any_multiline(args.query.unwrap_or_default()).into_string()?;
    let query_builder = mssql::QueryBuilder::new(&query_string);

    // TODO: bind paramters to query (-p)
    // TODO: handle streaming parameters (-s)

    let results = query_builder.execute(&mut conn).await?;

    // TODO: implement writing to files
    let mut out = BufWriter::new(std::io::stdout());
    for (set_idx, result_set) in results.into_iter().enumerate() {
        let res = match args.format.take().unwrap_or_default() {
            cli::OutputFormat::Json => writeln!(
                out,
                "{}",
                serde_json::to_string(&result_set.into_iter().collect::<Vec<_>>())?
            ),
            cli::OutputFormat::Text => {
                // TODO: use markdown table format
                writeln!(out, "result set {}:", set_idx + 1)?;
                for row in result_set {
                    writeln!(out, "> new row")?;
                    for (i, (col, val)) in row.iter_columns().zip(row.iter_values()).enumerate() {
                        writeln!(out, "{i}: {col} = {val}")?;
                    }
                }
                Ok(())
            }
        };
        match res {
            Err(e) if matches!(e.kind(), io::ErrorKind::BrokenPipe) => exit(0),
            _ => res?,
        }
    }
    Ok(())
}

async fn connect(args: cli::ArgsConnect) -> anyhow::Result<()> {
    Ok(())
}
