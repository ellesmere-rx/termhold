mod game;
mod ui;

fn main() {
    let colony_name = ui::run_welcome();
    let mut game = game::Game::new(colony_name);

    while !game.gameover {
        ui::render(&game);
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
