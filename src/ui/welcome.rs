//! Pre-game welcome screen: lore, commands, colony name.

use super::{WIDTH, clear_screen, read_input};

const DEFAULT_COLONY_NAME: &str = "Termhold";
const MAX_COLONY_NAME_LEN: usize = 28;
const WRAP: usize = WIDTH - 4;

const LORE: &[&str] = &[
    "The Old Empire is dead. No one remembers how it truly fell — war, famine, \
     plague, or the day the gods turned away. The world has been fading ever since.",
    "You have made camp at the edge of the known world. Build, survive, and hold \
     on — there is no wall behind you anymore.",
    "Settlers still speak of the Day of Silence. Some say God abandoned mankind. \
     Others that He died, sleeps, was slain, or never existed. No one knows. Only \
     that the fading did not stop.",
    "The frontier is not empty. Ruins predate the Empire. Forgotten roads vanish \
     into the wilds. In the long nights, something keeps watch.",
];

const GOAL: &str = "Hold your settlement for 365 days. Every command costs time.";

/// Show intro, read colony name. Empty input → [`DEFAULT_COLONY_NAME`].
pub fn run() -> String {
    clear_screen();

    println!("╔{}╗", "═".repeat(WIDTH - 2));
    println!("║ {:<width$} ║", "TERMHOLD", width = WIDTH - 4);
    println!("╚{}╝", "═".repeat(WIDTH - 2));
    println!();

    for paragraph in LORE {
        for line in wrap_text(paragraph, WRAP) {
            println!("  {line}");
        }
        println!();
    }

    for line in wrap_text(GOAL, WRAP) {
        println!("  {line}");
    }
    println!();

    println!(" COMMANDS");
    println!("{}", "─".repeat(WIDTH));
    println!("  g wood | stone | food   gather (1 day)");
    println!("  b hut | farm | …        build");
    println!("  w farm 2               workers (free turn)");
    println!("  y / n                  events · help · q quit");
    println!();

    println!("  Colony name [{DEFAULT_COLONY_NAME}] — Enter for default:");
    print!("  > ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    normalize_colony_name(read_input())
}

fn normalize_colony_name(input: String) -> String {
    let name = input.trim();
    if name.is_empty() {
        return DEFAULT_COLONY_NAME.to_string();
    }
    if name.len() <= MAX_COLONY_NAME_LEN {
        return name.to_string();
    }
    name[..MAX_COLONY_NAME_LEN].to_string()
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}
