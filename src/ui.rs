use std::io;

use crate::game::Commands;
use crate::game::Game;

const VISIBLE_LOGS: usize = 8;
const WIDTH: usize = 50;

pub fn clear_screen() {
    print!("\x1B[2J\x1B[H");
}

pub fn read_input() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn line() {
    println!("{}", "─".repeat(WIDTH));
}

fn title(text: &str) {
    println!(" {text}");
    line();
}

fn row(label: &str, value: impl std::fmt::Display) {
    println!("  {:<16} {value}", label);
}

fn action(cmd: &str, detail: &str) {
    println!("  {cmd:<22} {detail}");
}

fn signed_delta(value: isize) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        format!("{value}")
    }
}

fn parse_command(input: &str) -> Option<Commands> {
    let input = input.to_lowercase();
    let mut parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let verb = parts.remove(0);
    let target = parts.join(" ");

    match verb {
        "q" | "quit" | "exit" => Some(Commands::Quit),
        "g" | "gather" => parse_gather(&target),
        "b" | "build" => parse_build(&target),
        _ => None,
    }
}

fn parse_gather(target: &str) -> Option<Commands> {
    match target {
        "wood" => Some(Commands::GetWood),
        "stone" => Some(Commands::GetStone),
        "food" => Some(Commands::GetFood),
        _ => None,
    }
}

fn parse_build(target: &str) -> Option<Commands> {
    match target {
        "hut" => Some(Commands::BuildHut),
        "lumber" | "lumberyard" | "lumber-yard" | "lumber_yard" | "lumber yard" | "yard" => {
            Some(Commands::BuildLumberYard)
        }
        "quarry" | "stone-quarry" | "stonequarry" | "stone quarry" => {
            Some(Commands::BuildStoneQuarry)
        }
        "barn" => Some(Commands::BuildBarn),
        _ => None,
    }
}

pub fn render(game: &Game) {
    clear_screen();

    let colony = &game.colony;
    let balance = &game.balance;

    let wood_active = colony.wood_yield(balance);
    let stone_active = colony.stone_yield(balance);
    let food_potential = colony.food_yield(balance);
    let food_active = food_potential.min(colony.max_food.saturating_sub(colony.food));
    let passive_wood = colony.passive_wood(balance);
    let passive_stone = colony.passive_stone(balance);
    let upkeep = colony.population as isize;
    let food_net = food_active as isize - upkeep;

    println!("╔{}╗", "═".repeat(WIDTH - 2));
    println!(
        "║ {:<width$} ║",
        format!("{} — day {}", colony.name, game.world.days),
        width = WIDTH - 4
    );
    println!("╚{}╝", "═".repeat(WIDTH - 2));
    println!();

    title("RESOURCES");
    row("Population", format!("{} / {}", colony.population, colony.max_population));
    row("Wood", colony.wood);
    row("Stone", colony.stone);
    row("Food", format!("{} / {}", colony.food, colony.max_food));
    println!();

    title("BUILDINGS");
    row("Huts", colony.huts);
    row("Lumber yards", colony.lumber_yards);
    row("Stone quarries", colony.stone_quarries);
    row("Barns", colony.barns);
    println!();

    title("PER DAY (auto at tick)");
    row("Food upkeep", format!("{} food", signed_delta(-upkeep)));
    if passive_wood > 0 || passive_stone > 0 {
        row(
            "Passive",
            format!(
                "{} wood, {} stone",
                signed_delta(passive_wood as isize),
                signed_delta(passive_stone as isize)
            ),
        );
    }
    if colony.population + 1 <= colony.max_population {
        row(
            "Birth",
            format!(
                "{}% if food ≥ pop + {}",
                balance.birth_chance_percent, balance.population_increase_cost
            ),
        );
    }
    println!();

    title("COMMANDS (1 action = 1 day)");
    println!("  g <wood|stone|food>  |  b <hut|lumber|quarry|barn>  |  quit");
    println!();
    action("g wood", &format!("{} wood", signed_delta(wood_active as isize)));
    action("g stone", &format!("{} stone", signed_delta(stone_active as isize)));
    let food_detail = if food_active < food_potential {
        format!(
            "{} food (net {}, storage full)",
            signed_delta(food_active as isize),
            signed_delta(food_net)
        )
    } else {
        format!(
            "{} food (net {})",
            signed_delta(food_active as isize),
            signed_delta(food_net)
        )
    };
    action("g food", &food_detail);
    action(
        "b hut",
        &format!(
            "-{} wood, -{} stone, +{} max pop",
            balance.build_hut_wood_cost,
            balance.build_hut_stone_cost,
            balance.hut_max_population_increase
        ),
    );
    action(
        "b lumber",
        &format!(
            "-{} wood, -{} stone, +{} wood/day",
            balance.build_lumber_yard_wood_cost,
            balance.build_lumber_yard_stone_cost,
            balance.lumber_yard_wood_production
        ),
    );
    action(
        "b quarry",
        &format!(
            "-{} wood, -{} stone, +{} stone/day",
            balance.build_stone_quarry_wood_cost,
            balance.build_stone_quarry_stone_cost,
            balance.stone_quarry_stone_production
        ),
    );
    action(
        "b barn",
        &format!(
            "-{} wood, -{} stone, +{} food cap",
            balance.build_barn_wood_cost,
            balance.build_barn_stone_cost,
            balance.barn_max_food_storage_increase
        ),
    );
    println!();

    print_logs(game);
    println!();
    print!("> ");
    io::Write::flush(&mut io::stdout()).unwrap();
}

pub fn print_logs(game: &Game) {
    title("LOG");
    let start = game.logs.len().saturating_sub(VISIBLE_LOGS);
    if game.logs.is_empty() {
        println!("  (no events yet)");
        return;
    }
    for log in &game.logs[start..] {
        println!("  {log}");
    }
}

pub fn read_command() -> Option<Commands> {
    let input = read_input();
    if input.is_empty() {
        return None;
    }

    parse_command(&input).or_else(|| {
        println!("  Unknown command: \"{input}\"");
        println!("  Examples: g food | gather wood | b hut | build lumber | quit");
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gather_commands() {
        assert_eq!(parse_command("g wood"), Some(Commands::GetWood));
        assert_eq!(parse_command("gather stone"), Some(Commands::GetStone));
        assert_eq!(parse_command("G FOOD"), Some(Commands::GetFood));
    }

    #[test]
    fn parse_build_commands() {
        assert_eq!(parse_command("b hut"), Some(Commands::BuildHut));
        assert_eq!(parse_command("build lumber"), Some(Commands::BuildLumberYard));
        assert_eq!(parse_command("build lumber-yard"), Some(Commands::BuildLumberYard));
        assert_eq!(parse_command("build lumber yard"), Some(Commands::BuildLumberYard));
    }

    #[test]
    fn parse_quit_and_unknown() {
        assert_eq!(parse_command("quit"), Some(Commands::Quit));
        assert_eq!(parse_command("g"), None);
        assert_eq!(parse_command("gather"), None);
        assert_eq!(parse_command("b barn"), Some(Commands::BuildBarn));
    }
}
