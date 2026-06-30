use std::io;
// test comment
use crate::game::Commands;
use crate::game::Game;

const VISIBLE_LOGS: usize = 10;

pub fn clear_screen() {
    print!("\x1B[2J\x1B[H");
}

pub fn read_input() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub fn render(game: &Game) {
    clear_screen();

    // Main title
    println!(
        "Colony name: {} - day {}\n",
        game.colony.name, game.world.days
    );

    // State
    println!(
        "POP: {} | WOOD: {} | STONE: {} | FOOD: {}\n",
        game.colony.population, game.colony.wood, game.colony.stone, game.colony.food
    );

    // Some logs
    print_logs(game);

    // Dynamic hint render
    let wood_hint = format!(
        "w - wood (+{}, -{} food)",
        game.balance.gather_wood_base, game.balance.gather_wood_cost
    );
    let stone_hint = format!(
        "s - stone (+{}, -{} food)",
        game.balance.gather_stone_base, game.balance.gather_stone_cost
    );
    let food_hint = if game.balance.gather_food_cost == 0 {
        format!("f - food (+{})", game.balance.gather_food_base)
    } else {
        format!(
            "f - food (+{}, -{} food)",
            game.balance.gather_food_base, game.balance.gather_food_cost
        )
    };

    println!("{wood_hint} | {stone_hint} | {food_hint} | q - quit");
}

pub fn print_logs(game: &Game) {
    println!("- - - HISTORY - - -");
    let start = game.logs.len().saturating_sub(VISIBLE_LOGS);
    for log in &game.logs[start..] {
        println!("{log}");
    }
}

pub fn read_command() -> Option<Commands> {
    match read_input().as_str() {
        "w" => Some(Commands::GetWood),
        "s" => Some(Commands::GetStone),
        "f" => Some(Commands::GetFood),
        "q" => Some(Commands::Quit),
        _ => None,
    }
}
