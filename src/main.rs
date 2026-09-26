mod dialect;
mod schema;
mod text;
mod view;

#[cfg(test)]
mod property_tests;

use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process,
};

use clap::{ColorChoice, Parser};

use crate::dialect::Dialect;

/// Puu - Render JSON Schema for humans 🌳
#[derive(Parser)]
#[command(version)]
struct Args {
    /// JSON Schema file to render, or '-' to read from standard input
    schema: PathBuf,

    /// Override the JSON Schema draft
    #[arg(short, long, value_enum)]
    draft: Option<Dialect>,

    /// Show full annotations and uninterpreted values
    #[arg(short, long)]
    verbose: bool,

    /// Control colored tree output
    #[arg(short, long, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();
    configure_color(args.color);
    let input = read_input(&args.schema)?;
    let schema = schema::parse(&input, args.draft).map_err(|error| error.to_string())?;
    let document = view::from_schema(&schema, args.verbose);
    let tree = text::render(&document, &text::ColorTheme::default());
    write_output(&tree)
}

fn configure_color(color: ColorChoice) {
    match color {
        ColorChoice::Auto => {}
        ColorChoice::Always => console::set_colors_enabled(true),
        ColorChoice::Never => console::set_colors_enabled(false),
    }
}

fn read_input(path: &Path) -> Result<String, String> {
    if path == Path::new("-") {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("cannot read standard input: {error}"))?;
        Ok(input)
    } else {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

fn write_output(output: &str) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(output.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(format!("cannot write standard output: {error}")),
    }
}
