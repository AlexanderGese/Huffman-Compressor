//! huffman — a Huffman file compressor.

mod bitio;
mod codec;
mod huffman;
mod tui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use tui::Stats;

#[derive(Parser)]
#[command(name = "huffman", version, about = "A Huffman file compressor")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Compress a file to <input>.huff
    Compress { input: PathBuf, #[arg(short)] output: Option<PathBuf> },
    /// Restore a .huff file
    Decompress { input: PathBuf, #[arg(short)] output: Option<PathBuf> },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let r = match &cli.cmd {
        Cmd::Compress { input, output } => do_compress(input, output.as_ref()),
        Cmd::Decompress { input, output } => do_decompress(input, output.as_ref()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("\x1b[31merror:\x1b[0m {e}"); ExitCode::FAILURE }
    }
}

fn do_compress(input: &PathBuf, output: Option<&PathBuf>) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("reading {}: {e}", input.display()))?;
    let packed = codec::compress(&data);
    let out = output.cloned().unwrap_or_else(|| PathBuf::from(format!("{}.huff", input.display())));
    std::fs::write(&out, &packed).map_err(|e| format!("writing {}: {e}", out.display()))?;
    if !data.is_empty() {
        let freq = huffman::freq_table(&data);
        let tree = huffman::build_tree(&freq).unwrap();
        let codes = huffman::gen_codes(&tree);
        let stats = Stats::compute(data.len(), packed.len(), &freq, &codes);
        let title = input.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "input".into());
        tui::print_view(&title, &tree, &codes, &stats);
    }
    Ok(())
}

fn do_decompress(input: &PathBuf, output: Option<&PathBuf>) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("reading {}: {e}", input.display()))?;
    let restored = codec::decompress(&data)?;
    let out = output.cloned().unwrap_or_else(|| {
        let s = input.to_string_lossy();
        if let Some(st) = s.strip_suffix(".huff") { PathBuf::from(st.to_string()) } else { PathBuf::from(format!("{s}.decoded")) }
    });
    std::fs::write(&out, &restored).map_err(|e| format!("writing {}: {e}", out.display()))?;
    eprintln!("restored {} bytes -> {}", restored.len(), out.display());
    Ok(())
}
