//! huffman — a Huffman file compressor with a visual terminal UI.

mod bitio;
mod codec;
mod huffman;
mod tui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use tui::Stats;

#[derive(Parser)]
#[command(name = "huffman", version, about = "A Huffman file compressor with a visual CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    /// Render the view to stdout (ANSI) instead of the interactive TUI.
    #[arg(long, global = true)]
    print: bool,
    /// Skip the visual view, print only a one-line summary.
    #[arg(long, global = true)]
    no_tui: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Compress a file to <input>.huff
    Compress {
        input: PathBuf,
        #[arg(short)]
        output: Option<PathBuf>,
    },
    /// Restore a .huff file
    Decompress {
        input: PathBuf,
        #[arg(short)]
        output: Option<PathBuf>,
    },
    /// Compress a built-in sample and show the tree
    Demo,
}

const SAMPLE: &str = "huffman coding builds a tree from symbol frequencies, then \
gives the most common symbols the shortest bit patterns. the more skewed the \
distribution, the better it compresses.";

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.cmd {
        Cmd::Compress { input, output } => do_compress(input, output.as_ref(), &cli),
        Cmd::Decompress { input, output } => do_decompress(input, output.as_ref()),
        Cmd::Demo => {
            show("sample", SAMPLE.as_bytes(), &codec::compress(SAMPLE.as_bytes()), &cli);
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m {e}");
            ExitCode::FAILURE
        }
    }
}

fn do_compress(input: &PathBuf, output: Option<&PathBuf>, cli: &Cli) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("reading {}: {e}", input.display()))?;
    let packed = codec::compress(&data);

    // round-trip verify before we trust the archive
    let check = codec::decompress(&packed)?;
    if check != data {
        return Err("round-trip verification FAILED — refusing to write a corrupt archive".into());
    }

    let out = output
        .cloned()
        .unwrap_or_else(|| PathBuf::from(format!("{}.huff", input.display())));
    std::fs::write(&out, &packed).map_err(|e| format!("writing {}: {e}", out.display()))?;

    let title = input
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "input".into());
    eprintln!(
        "\x1b[32m\u{2713}\x1b[0m {} \u{2192} {}  ({} \u{2192} {} bytes, verified)",
        input.display(),
        out.display(),
        data.len(),
        packed.len()
    );
    show(&title, &data, &packed, cli);
    Ok(())
}

fn do_decompress(input: &PathBuf, output: Option<&PathBuf>) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("reading {}: {e}", input.display()))?;
    let restored = codec::decompress(&data)?;
    let out = output.cloned().unwrap_or_else(|| {
        let s = input.to_string_lossy();
        if let Some(stripped) = s.strip_suffix(".huff") {
            PathBuf::from(stripped.to_string())
        } else {
            PathBuf::from(format!("{s}.decoded"))
        }
    });
    std::fs::write(&out, &restored).map_err(|e| format!("writing {}: {e}", out.display()))?;
    eprintln!(
        "\x1b[32m\u{2713}\x1b[0m {} \u{2192} {}  ({} bytes)",
        input.display(),
        out.display(),
        restored.len()
    );
    Ok(())
}

fn show(title: &str, data: &[u8], packed: &[u8], cli: &Cli) {
    if data.is_empty() {
        println!("(empty input — nothing to visualize)");
        return;
    }
    let freq = huffman::freq_table(data);
    let tree = huffman::build_tree(&freq).expect("non-empty data has a tree");
    let codes = huffman::gen_codes(&tree);
    let stats = Stats::compute(data.len(), packed.len(), &freq, &codes);

    if cli.no_tui {
        println!(
            "{title}: {} \u{2192} {} bytes ({:.1}% saved, {} symbols)",
            stats.orig,
            stats.comp,
            stats.saved_pct(),
            stats.symbols
        );
    } else if cli.print {
        tui::print_view(title, &tree, &codes, &stats);
    } else if let Err(e) = tui::tui_view(title, &tree, &codes, &stats) {
        eprintln!("(tui unavailable: {e}; printing instead)");
        tui::print_view(title, &tree, &codes, &stats);
    }
}
