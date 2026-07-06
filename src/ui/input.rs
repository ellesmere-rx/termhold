//! CLI text → [`Actions`](crate::game::Actions).

use crate::game::Actions;
use crate::game::BuildingKind;
use crate::game::ResourceKind;

/// Parsed player input line.
#[derive(Debug, PartialEq)]
pub enum ActionInput {
    Action(Actions),
    Help,
    Invalid,
    Empty,
}

pub fn classify_input(input: &str) -> ActionInput {
    if input.is_empty() {
        return ActionInput::Empty;
    }
    if is_help_input(input) {
        return ActionInput::Help;
    }
    match parse_line(input) {
        Some(action) => ActionInput::Action(action),
        None => ActionInput::Invalid,
    }
}

pub fn is_help_input(input: &str) -> bool {
    matches!(input.trim().to_lowercase().as_str(), "help" | "?" | "h")
}

pub fn parse_line(input: &str) -> Option<Actions> {
    let input = input.to_lowercase();
    let mut parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let verb = parts.remove(0);
    let target = parts.join(" ");

    match verb {
        "q" | "quit" | "exit" => Some(Actions::Quit),
        "g" | "gather" => parse_gather(&target),
        "b" | "build" => parse_build(&target),
        "w" | "workers" | "work" => parse_workers(&target),
        _ => None,
    }
}

fn parse_gather(target: &str) -> Option<Actions> {
    parse_resource_target(target).map(Actions::Gather)
}

fn parse_build(target: &str) -> Option<Actions> {
    parse_building_target(target).map(Actions::Build)
}

fn parse_workers(input: &str) -> Option<Actions> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let count: usize = parts.last()?.parse().ok()?;
    let building = parts[..parts.len() - 1].join(" ");
    let kind = parse_production_target(&building)?;

    Some(Actions::SetWorkers { kind, count })
}

fn parse_resource_target(s: &str) -> Option<ResourceKind> {
    match s {
        "wood" => Some(ResourceKind::Wood),
        "stone" => Some(ResourceKind::Stone),
        "food" => Some(ResourceKind::Food),
        _ => None,
    }
}

fn parse_building_target(s: &str) -> Option<BuildingKind> {
    match s {
        "hut" => Some(BuildingKind::Hut),
        "barn" => Some(BuildingKind::Barn),
        "farm" => Some(BuildingKind::Farm),
        "lumber" | "lumberyard" | "lumber-yard" | "lumber_yard" | "lumber yard" | "yard" => {
            Some(BuildingKind::LumberYard)
        }
        "quarry" | "stone-quarry" | "stonequarry" | "stone quarry" => {
            Some(BuildingKind::StoneQuarry)
        }
        _ => None,
    }
}

fn parse_production_target(s: &str) -> Option<BuildingKind> {
    parse_building_target(s).filter(|kind| kind.employs_workers())
}
