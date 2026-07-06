mod game;
mod ui;

fn main() {
    println!("[ Start ]");

    let mut game = game::Game::default();

    while !game.gameover {
        ui::render(&game);
        // While an event is pending, invalid/empty input must not advance the day.
        let pending_event = game.pending_event.is_some();
        match ui::read_action(pending_event) {
            ui::ActionInput::Action(action) => {
                if action == game::Actions::Quit {
                    break;
                }
                let free_turn = action.is_free_turn();
                game.process_action(action);
                if !free_turn {
                    game.tick();
                }
            }
            ui::ActionInput::Help => {
                ui::show_help(&game);
            }
            ui::ActionInput::Invalid => {
                if pending_event {
                    game.logs("Answer the event: y or n.".into());
                } else {
                    game.logs(ui::INVALID_COMMAND_MSG.into());
                    game.tick();
                }
            }
            ui::ActionInput::Empty => {
                if pending_event {
                    game.logs("Answer the event: y or n.".into());
                } else {
                    game.logs(ui::EMPTY_COMMAND_MSG.into());
                    game.tick();
                }
            }
        }
    }

    println!("[ End ]");
}
