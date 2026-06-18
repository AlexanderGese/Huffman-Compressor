//! Rendering: the ASCII Huffman tree, a stats panel, and a codes legend —
//! available both as a plain colored stdout dump (`--print`) and an interactive
//! ratatui view.

use crate::huffman::Node;
use std::collections::HashMap;

/// Compression statistics for the header panel.
pub struct Stats {
    pub orig: usize,
    pub comp: usize,
    pub symbols: usize,
    pub entropy: f64,
    pub avg_len: f64,
}

impl Stats {
    pub fn compute(orig: usize, comp: usize, freq: &[u64; 256], codes: &HashMap<u8, Vec<bool>>) -> Self {
        let total: u64 = freq.iter().sum();
        let mut entropy = 0.0;
        let mut weighted_len = 0.0;
        let mut symbols = 0;
        for (b, &f) in freq.iter().enumerate() {
            if f == 0 {
                continue;
            }
            symbols += 1;
            let p = f as f64 / total as f64;
            entropy -= p * p.log2();
            if let Some(code) = codes.get(&(b as u8)) {
                weighted_len += p * code.len() as f64;
            }
        }
        Stats {
            orig,
            comp,
            symbols,
            entropy,
            avg_len: weighted_len,
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.orig == 0 {
            0.0
        } else {
            self.comp as f64 / self.orig as f64
        }
    }

    pub fn saved_pct(&self) -> f64 {
        (1.0 - self.ratio()) * 100.0
    }
}

fn display_byte(b: u8) -> String {
    match b {
        b' ' => "' '".into(),
        b'\n' => "'\\n'".into(),
        b'\t' => "'\\t'".into(),
        b'\r' => "'\\r'".into(),
        0x21..=0x7e => format!("'{}'", b as char),
        _ => format!("0x{:02x}", b),
    }
}

fn code_str(code: &[bool]) -> String {
    code.iter().map(|&b| if b { '1' } else { '0' }).collect()
}

/// One printed line of the tree.
struct Row {
    prefix: String,
    edge: String,
    leaf: Option<(String, u64, String)>, // (display byte, freq, code)
    weight: Option<u64>,
}

fn build_rows(
    node: &Node,
    prefix: &str,
    is_root: bool,
    is_left: bool,
    is_last: bool,
    codes: &HashMap<u8, Vec<bool>>,
    rows: &mut Vec<Row>,
) {
    let edge = if is_root {
        String::new()
    } else {
        format!("{}\u{2500}{}\u{2500} ", if is_last { '\u{2514}' } else { '\u{251c}' }, if is_left { '0' } else { '1' })
    };
    match node {
        Node::Leaf { byte, freq } => {
            let code = codes.get(byte).map(|c| code_str(c)).unwrap_or_default();
            rows.push(Row {
                prefix: prefix.to_string(),
                edge,
                leaf: Some((display_byte(*byte), *freq, code)),
                weight: None,
            });
        }
        Node::Internal { freq, left, right } => {
            rows.push(Row {
                prefix: prefix.to_string(),
                edge,
                leaf: None,
                weight: Some(*freq),
            });
            let child_prefix = if is_root {
                prefix.to_string()
            } else {
                format!("{}{}", prefix, if is_last { "    " } else { "\u{2502}   " })
            };
            build_rows(left, &child_prefix, false, true, false, codes, rows);
            build_rows(right, &child_prefix, false, false, true, codes, rows);
        }
    }
}

fn tree_rows(root: &Node, codes: &HashMap<u8, Vec<bool>>) -> Vec<Row> {
    let mut rows = Vec::new();
    build_rows(root, "", true, false, true, codes, &mut rows);
    rows
}

// ---- ANSI (stdout) rendering -------------------------------------------------

const DIM: &str = "\x1b[90m";
const YEL: &str = "\x1b[33m";
const GRN: &str = "\x1b[1;32m";
const CYN: &str = "\x1b[36m";
const MAG: &str = "\x1b[35m";
const BOLD: &str = "\x1b[1m";
const RST: &str = "\x1b[0m";

fn row_ansi(r: &Row) -> String {
    let body = match &r.leaf {
        Some((disp, freq, code)) => format!(
            "{GRN}{disp}{RST} {DIM}\u{00d7}{freq}{RST}  {CYN}\u{2192} {code}{RST}"
        ),
        None => format!("{MAG}[\u{25cf}]{RST} {DIM}{}{RST}", r.weight.unwrap_or(0)),
    };
    format!("{DIM}{}{RST}{YEL}{}{RST}{}", r.prefix, r.edge, body)
}

pub fn print_view(title: &str, root: &Node, codes: &HashMap<u8, Vec<bool>>, stats: &Stats) {
    println!();
    println!("  {BOLD}{MAG}\u{1f333} Huffman Compressor{RST} {DIM}\u{2014}{RST} {BOLD}{title}{RST}");
    println!("  {DIM}{}{RST}", "\u{2500}".repeat(56));
    for r in tree_rows(root, codes) {
        println!("  {}", row_ansi(&r));
    }
    println!("  {DIM}{}{RST}", "\u{2500}".repeat(56));
    let bar = ratio_bar(stats.saved_pct());
    println!(
        "  {DIM}original{RST}   {BOLD}{}{RST} B      {DIM}compressed{RST} {BOLD}{}{RST} B",
        stats.orig, stats.comp
    );
    println!(
        "  {DIM}saved{RST}      {GRN}{:.1}%{RST}  {bar}  {DIM}(ratio {:.3}){RST}",
        stats.saved_pct(),
        stats.ratio()
    );
    println!(
        "  {DIM}symbols{RST}    {CYN}{}{RST}        {DIM}entropy{RST} {CYN}{:.3}{RST} {DIM}bits/sym{RST}  {DIM}avg code{RST} {CYN}{:.3}{RST}",
        stats.symbols, stats.entropy, stats.avg_len
    );
    println!();
}

fn ratio_bar(pct: f64) -> String {
    let filled = ((pct / 100.0) * 24.0).round().clamp(0.0, 24.0) as usize;
    format!(
        "{GRN}{}{DIM}{}{RST}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(24 - filled)
    )
}

// ---- interactive ratatui rendering ------------------------------------------

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

fn row_line(r: &Row) -> Line<'static> {
    let mut spans = vec![
        Span::styled(r.prefix.clone(), Style::default().fg(Color::DarkGray)),
        Span::styled(r.edge.clone(), Style::default().fg(Color::Yellow)),
    ];
    match &r.leaf {
        Some((disp, freq, code)) => {
            spans.push(Span::styled(disp.clone(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(format!(" \u{00d7}{freq}"), Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(format!("  \u{2192} {code}"), Style::default().fg(Color::Cyan)));
        }
        None => {
            spans.push(Span::styled("[\u{25cf}]", Style::default().fg(Color::Magenta)));
            spans.push(Span::styled(format!(" {}", r.weight.unwrap_or(0)), Style::default().fg(Color::DarkGray)));
        }
    }
    Line::from(spans)
}

pub fn tui_view(title: &str, root: &Node, codes: &HashMap<u8, Vec<bool>>, stats: &Stats) -> io::Result<()> {
    let mut tree_lines: Vec<Line> = tree_rows(root, codes).iter().map(row_line).collect();
    tree_lines.insert(0, Line::from(""));

    let stat_lines = vec![
        Line::from(vec![
            Span::styled("original ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} B", stats.orig), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled("compressed ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} B", stats.comp), Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("saved    ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1}%", stats.saved_pct()), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(format!("   ratio {:.3}", stats.ratio()), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("symbols ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", stats.symbols), Style::default().fg(Color::Cyan)),
            Span::styled("   entropy ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.3}", stats.entropy), Style::default().fg(Color::Cyan)),
            Span::styled(" bits/sym   avg code ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.3}", stats.avg_len), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![Span::styled("press q to quit", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))]),
    ];

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let title_owned = format!(" \u{1f333} Huffman Compressor \u{2014} {title} ");
    let draw = |terminal: &mut Terminal<CrosstermBackend<io::Stdout>>| -> io::Result<()> {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(6)])
                .split(f.area());
            let tree = Paragraph::new(tree_lines.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(title_owned.clone(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
            );
            f.render_widget(tree, chunks[0]);
            let panel = Paragraph::new(stat_lines.clone()).block(
                Block::default().borders(Borders::ALL).title(" stats "),
            );
            f.render_widget(panel, chunks[1]);
        })?;
        Ok(())
    };

    draw(&mut terminal)?;
    loop {
        if let Event::Key(k) = event::read()? {
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => break,
                _ => draw(&mut terminal)?,
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
