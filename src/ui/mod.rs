//! Terminal UI: render + input.

mod input;

use std::io;

use crate::game::BuildingKind;
use crate::game::Game;

pub use input::ActionInput;

const WIDTH: usize = 80;
const VISIBLE_LOGS: usize = 10;
const LABEL_W: usize = 12;

pub fn clear_screen() {
    print!("\x1B[2J\x1B[H");
}

pub fn read_input() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn hr() {
    println!("{}", "─".repeat(WIDTH));
}

fn section(title: &str) {
    println!(" {title}");
    hr();
}

fn row(label: &str, value: impl std::fmt::Display) {
    println!("  {label:<LABEL_W$} {value}");
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn signed_delta(value: isize) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        format!("{value}")
    }
}

fn render_build_menu(balance: &crate::game::Balance) {
    section("BUILD");
    row(
        "b hut",
        format!(
            "{}w {}s → +{} pop",
            balance.buildings.build_hut_wood_cost,
            balance.buildings.build_hut_stone_cost,
            balance.buildings.hut_max_population_increase,
        ),
    );
    row(
        "b farm",
        format!(
            "{}w {}s → +{} food/d, {} workers",
            balance.buildings.build_farm_wood_cost,
            balance.buildings.build_farm_stone_cost,
            balance.buildings.farm_food_production,
            balance.buildings.farm_max_workers,
        ),
    );
    row(
        "b lumber",
        format!(
            "{}w {}s → +{} wood/d, {} workers",
            balance.buildings.build_lumber_yard_wood_cost,
            balance.buildings.build_lumber_yard_stone_cost,
            balance.buildings.lumber_yard_wood_production,
            balance.buildings.lumber_yard_max_workers,
        ),
    );
    row(
        "b quarry",
        format!(
            "{}w {}s → +{} stone/d, {} workers",
            balance.buildings.build_stone_quarry_wood_cost,
            balance.buildings.build_stone_quarry_stone_cost,
            balance.buildings.stone_quarry_stone_production,
            balance.buildings.stone_quarry_max_workers,
        ),
    );
    row(
        "b barn",
        format!(
            "{}w {}s → +{} food cap",
            balance.buildings.build_barn_wood_cost,
            balance.buildings.build_barn_stone_cost,
            balance.buildings.barn_max_food_storage_increase,
        ),
    );
    println!();
}

fn gather_short(yield_amount: usize, free: usize, abbrev: &str) -> String {
    if free == 0 {
        "blocked".to_string()
    } else {
        format!("{}{abbrev}", signed_delta(yield_amount as isize))
    }
}

fn food_gather_hint(
    food_active: usize,
    food_potential: usize,
    food_net: isize,
    food: usize,
    max_food: usize,
    free: usize,
) -> String {
    if free == 0 {
        return "blocked".to_string();
    }

    let space = max_food.saturating_sub(food);
    if space == 0 {
        return format!("blocked (storage {food}/{max_food})");
    }

    let yield_part = format!("{}{}", signed_delta(food_active as isize), " food");
    let net_part = format!("net {}", signed_delta(food_net));

    if food_active < food_potential {
        format!("{yield_part} ({net_part}, +{space} cap left)")
    } else {
        format!("{yield_part} ({net_part})")
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
    let passive_food = colony.passive_food(balance);
    // Tick order: eat first, then passive — not current storage headroom.
    let food_after_upkeep = colony.food.saturating_sub(colony.population);
    let passive_food_effective =
        passive_food.min(colony.max_food.saturating_sub(food_after_upkeep));
    let upkeep = colony.population as isize;
    let food_after_gather = (colony.food + food_active).min(colony.max_food);
    let food_after_gather_upkeep = food_after_gather.saturating_sub(colony.population);
    let passive_food_if_gather =
        passive_food.min(colony.max_food.saturating_sub(food_after_gather_upkeep));
    let food_net_if_gather = food_active as isize + passive_food_if_gather as isize - upkeep;
    let food_net_passive = passive_food_effective as isize - upkeep;
    let free = colony.free_workers();
    let assigned = colony.assigned_workers();

    println!("╔{}╗", "═".repeat(WIDTH - 2));
    println!(
        "║ {:<width$} ║",
        format!(
            "{} — day {} / {}",
            colony.name,
            game.world.days,
            crate::game::WIN_DAY
        ),
        width = WIDTH - 4
    );
    println!("╚{}╝", "═".repeat(WIDTH - 2));
    println!();

    section("COLONY");
    let pop = if colony.population >= colony.max_population {
        format!(
            "{}/{} (housing full)",
            colony.population, colony.max_population
        )
    } else {
        format!("{}/{}", colony.population, colony.max_population)
    };
    row("Population", pop);
    row("Workers", format!("{assigned} on jobs, {free} free"));
    row("Wood", colony.wood);
    row("Stone", colony.stone);
    row("Food", format!("{} / {}", colony.food, colony.max_food));
    println!();

    section("BUILDINGS");
    row("Huts", colony.count(BuildingKind::Hut));
    row("Barns", colony.count(BuildingKind::Barn));
    if colony.count(BuildingKind::Farm) > 0 {
        row(
            "Farms",
            format!(
                "{} — {}/{} workers, {} staffed (+{} food/day)",
                colony.count(BuildingKind::Farm),
                colony.workers_on(BuildingKind::Farm),
                colony.workers_needed(BuildingKind::Farm, balance),
                colony.staffed(BuildingKind::Farm, balance),
                balance.buildings.farm_food_production
                    * colony.staffed(BuildingKind::Farm, balance),
            ),
        );
    }
    if colony.count(BuildingKind::LumberYard) > 0 {
        row(
            "Lumber yards",
            format!(
                "{} — {}/{} workers, {} staffed (+{} wood/day)",
                colony.count(BuildingKind::LumberYard),
                colony.workers_on(BuildingKind::LumberYard),
                colony.workers_needed(BuildingKind::LumberYard, balance),
                colony.staffed(BuildingKind::LumberYard, balance),
                balance.buildings.lumber_yard_wood_production
                    * colony.staffed(BuildingKind::LumberYard, balance),
            ),
        );
    }
    if colony.count(BuildingKind::StoneQuarry) > 0 {
        row(
            "Quarries",
            format!(
                "{} — {}/{} workers, {} staffed (+{} stone/day)",
                colony.count(BuildingKind::StoneQuarry),
                colony.workers_on(BuildingKind::StoneQuarry),
                colony.workers_needed(BuildingKind::StoneQuarry, balance),
                colony.staffed(BuildingKind::StoneQuarry, balance),
                balance.buildings.stone_quarry_stone_production
                    * colony.staffed(BuildingKind::StoneQuarry, balance),
            ),
        );
    }
    if !colony.buildings.has_production() {
        row("Production", "none — build farm, lumber, or quarry");
    }
    println!();

    section("PER DAY");
    row("Food upkeep", format!("{} food", signed_delta(-upkeep)));
    if passive_wood > 0 || passive_stone > 0 || passive_food_effective > 0 {
        row(
            "Passive",
            format!(
                "{} wood, {} stone, {} food",
                signed_delta(passive_wood as isize),
                signed_delta(passive_stone as isize),
                signed_delta(passive_food_effective as isize),
            ),
        );
    }
    row(
        "Food net",
        format!("{} without gather", signed_delta(food_net_passive)),
    );
    if colony.population < colony.max_population {
        row(
            "Birth chance",
            format!(
                "{}% if food ≥ pop + {}",
                balance.population.birth_chance_percent, balance.population.increase_cost
            ),
        );
    }
    println!();

    render_build_menu(balance);

    section("ACTIONS");
    row("g wood", gather_short(wood_active, free, " wood"));
    row("g stone", gather_short(stone_active, free, " stone"));
    row(
        "g food",
        food_gather_hint(
            food_active,
            food_potential,
            food_net_if_gather,
            colony.food,
            colony.max_food,
            free,
        ),
    );
    row("w / help", "free — no day pass");
    println!();

    print_logs(game);
    println!();
    print!("> ");
    io::Write::flush(&mut io::stdout()).unwrap();
}

pub fn show_help(game: &Game) {
    clear_screen();

    let colony = &game.colony;
    let balance = &game.balance;

    println!("=== HELP ===");
    println!();
    println!("Timing:");
    println!("  g, b — cost 1 day");
    println!("  w, help — free (no day pass)");
    println!();
    println!("Gather (g):");
    println!("  g wood / g stone / g food");
    println!("  Uses free settlers only; blocked at 0 free");
    println!();
    println!("Build (b):");
    println!(
        "  b hut     -{}w -{}s  +{} max pop",
        balance.buildings.build_hut_wood_cost,
        balance.buildings.build_hut_stone_cost,
        balance.buildings.hut_max_population_increase
    );
    println!(
        "  b farm    -{}w -{}s  +{} food/day ({} workers/farm)",
        balance.buildings.build_farm_wood_cost,
        balance.buildings.build_farm_stone_cost,
        balance.buildings.farm_food_production,
        balance.buildings.farm_max_workers
    );
    println!(
        "  b lumber  -{}w -{}s  +{} wood/day ({} workers/yard)",
        balance.buildings.build_lumber_yard_wood_cost,
        balance.buildings.build_lumber_yard_stone_cost,
        balance.buildings.lumber_yard_wood_production,
        balance.buildings.lumber_yard_max_workers
    );
    println!(
        "  b quarry  -{}w -{}s  +{} stone/day ({} workers/quarry)",
        balance.buildings.build_stone_quarry_wood_cost,
        balance.buildings.build_stone_quarry_stone_cost,
        balance.buildings.stone_quarry_stone_production,
        balance.buildings.stone_quarry_max_workers
    );
    println!(
        "  b barn    -{}w -{}s  +{} food storage",
        balance.buildings.build_barn_wood_cost,
        balance.buildings.build_barn_stone_cost,
        balance.buildings.barn_max_food_storage_increase
    );
    println!();
    println!("Workers (w):");
    if colony.count(BuildingKind::Farm) > 0 {
        println!(
            "  w farm 0..{}",
            colony.workers_needed(BuildingKind::Farm, balance)
        );
    }
    if colony.count(BuildingKind::LumberYard) > 0 {
        println!(
            "  w lumber 0..{}",
            colony.workers_needed(BuildingKind::LumberYard, balance)
        );
    }
    if colony.count(BuildingKind::StoneQuarry) > 0 {
        println!(
            "  w quarry 0..{}",
            colony.workers_needed(BuildingKind::StoneQuarry, balance)
        );
    }
    println!("  w auto — fill farm → lumber → quarry once");
    println!();
    print!("Press Enter to continue...");
    io::Write::flush(&mut io::stdout()).unwrap();
    let _ = read_input();
}

pub fn print_logs(game: &Game) {
    section("LOG");
    let start = game.logs.len().saturating_sub(VISIBLE_LOGS);
    let max = WIDTH - 4;
    if game.logs.is_empty() {
        println!("  (no events yet)");
        return;
    }
    for log in &game.logs[start..] {
        println!("  {}", clip(log, max));
    }
}

pub const INVALID_COMMAND_MSG: &str = "The settlers did not understand what you wanted to say and spent the whole day in contemplation.";

pub const EMPTY_COMMAND_MSG: &str = "The settlers, like you, decided to do nothing today.";

pub fn read_action() -> ActionInput {
    input::classify_input(&read_input())
}
