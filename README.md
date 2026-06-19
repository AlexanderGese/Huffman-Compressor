# 🌳 Huffman Compressor

[![crates.io](https://img.shields.io/crates/v/huffman-tui.svg)](https://crates.io/crates/huffman-tui)

A small, from-scratch **Huffman file compressor** in Rust with a visual terminal UI.
It builds the Huffman tree from a file's byte frequencies, assigns the shortest
bit-codes to the most common bytes, and shows you the whole tree.

**▶ Try it live (no install): https://alexandergese.github.io/Huffman-Compressor/**

![the tree view](docs/screenshot.png)

## Features

- Lossless compress / decompress of **any file** (text or binary).
- Self-describing `.huff` container — the Huffman tree is serialized into the
  archive, so decoding rebuilds the *exact* tree (no ambiguity).
- **Round-trip verified** on compress: it decodes the archive in memory and
  refuses to write anything that doesn't match the input byte-for-byte.
- A colored ASCII **Huffman tree** view, plus stats: compression ratio, entropy
  (bits/symbol), average code length, and the per-character codes.
- Handles the awkward edge cases: empty files and single-symbol files.

## Install

The quickest way — install the published crate from [crates.io](https://crates.io/crates/huffman-tui):

```sh
cargo install huffman-tui
```

That puts a `huffman` binary on your `PATH`. You'll need the Rust toolchain
first; if you don't have it, install it once with
[rustup](https://rustup.rs):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Prefer not to install anything? The [live web version](https://alexandergese.github.io/Huffman-Compressor/)
runs entirely in the browser, or you can [build from source](#build).

## Usage

```sh
# compress a file -> file.huff (and show the tree)
huffman compress notes.txt

# restore it
huffman decompress notes.txt.huff -o notes.restored.txt

# see the tree for a built-in sample
huffman demo

# render the view straight to stdout (great for screenshots / piping)
huffman demo --print

# just the numbers, no visual
huffman compress notes.txt --no-tui
```

## `.huff` format

```
"HUF1"        4 bytes   magic
original_len  u64 LE    number of bytes in the original
tree_nbits    u32 LE    bit-length of the serialized tree
tree          bytes     pre-order tree: 0 = internal (L,R), 1 = leaf + 8-bit byte
payload_pad   u8        padding bits in the last payload byte
payload       bytes     the Huffman bitstream
```

## Build

```sh
cargo build --release
cargo test
```

Built with `ratatui` + `crossterm` for the UI and `clap` for the CLI.
