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
    time::Duration,
};

use clap::{ColorChoice, Parser};

use crate::dialect::Dialect;

/// Puu - Render JSON Schema for humans 🌳
#[derive(Parser)]
#[command(version)]
struct Args {
    /// JSON Schema file or URL to render, or '-' to read from standard input
    schema: PathBuf,

    /// Override the JSON Schema draft
    #[arg(short, long, value_enum)]
    draft: Option<Dialect>,

    /// Render the schema at a JSON Pointer
    #[arg(short, long)]
    pointer: Option<String>,

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
    let parsed = schema::parse(&input, args.draft).map_err(|error| error.to_string())?;
    let index = schema::SchemaIndex::new(&parsed);
    let root = match args.pointer.as_deref() {
        Some(pointer) => index.select(pointer).map_err(|error| error.to_string())?,
        None => index.get("#").expect("the document root should be indexed"),
    };
    let options = view::ViewOptions {
        verbose: args.verbose,
        ..view::ViewOptions::default()
    };
    let document = view::from_schema(
        &parsed.root,
        root.schema,
        &root.display_name,
        args.pointer.is_some(),
        &options,
    );
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
    } else if let Some(url) = path
        .to_str()
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
    {
        read_url(url)
    } else {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

fn read_url(url: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("cannot read {url}: {error}"))?;

    response
        .body_mut()
        .with_config()
        .limit(10 * 1024 * 1024)
        .read_to_string()
        .map_err(|error| format!("cannot read {url}: {error}"))
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
