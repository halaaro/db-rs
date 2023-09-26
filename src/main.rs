#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::panic)]
#![warn(clippy::todo)]

mod cli;
mod fmt_util;
use cli::ArgGet;
mod mssql;

use std::{
    io::{BufWriter, Write, self},
    process::exit,
};

const EXIT_ARG_ERROR: i32 = 2;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    // TODO: replace with a require_only_one method
    if args
        .iter()
        .filter(|a| matches!(a.as_str(), "-q" | "-c"))
        .count()
        > 1
    {
        eprintln!("Only one of -q -c allowed");
        exit(EXIT_ARG_ERROR);
    }

    // TODO: handle background connections (-d)
    let conn_string = cli::Source::new_any_line(args.get_required("-x")).into_string()?;
    let mut conn = mssql::Connection::from_string(&conn_string).await?;

    let query_string = cli::Source::new_any_multiline(args.get_required("-q")).into_string()?;
    let query_builder = mssql::QueryBuilder::new(&query_string);

    // TODO: bind paramters to query (-p)
    // TODO: handle streaming parameters (-s)

    let results = query_builder.execute(&mut conn).await?;

    let output_format = args.get("-f").unwrap_or_else(|| "json".to_string());
    if !["json", "text"].iter().any(|s| s == &output_format) {
        eprintln!("invalid format: {output_format}, valid values: json, text");
        exit(EXIT_ARG_ERROR);
    }

    // TODO: implement writing to files
    let mut out = BufWriter::new(std::io::stdout());
    for (set_idx, result_set) in results.into_iter().enumerate() {
        let res = match output_format.as_str() {
            "json" => writeln!(
                out,
                "{}",
                serde_json::to_string(&result_set.into_iter().collect::<Vec<_>>())?
            ),
            "text" => {
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
            _ => unreachable!("invalid output format"),
        };
        match res {
            Err(e) if matches!(e.kind(), io::ErrorKind::BrokenPipe) => exit(0),
            _ => res?,
        }
    }
    Ok(())
}
