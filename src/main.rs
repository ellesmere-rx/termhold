mod game;
mod ui;

fn main() {
    println!("[ Start ]");

    let mut game = game::Game::default();

    while !game.gameover {
        ui::render(&game);
        match ui::read_action() {
            ui::ActionInput::Action(action) => {
                if action == game::Actions::Quit {
                    break;
                }
                let is_worker = action.is_worker_management();
                game.process_action(action);
                if !is_worker {
                    game.tick();
                }
            }
            ui::ActionInput::Help => {
                ui::show_help(&game);
            }
            ui::ActionInput::Invalid => {
                game.logs(ui::INVALID_COMMAND_MSG.into());
                game.tick();
            }
            ui::ActionInput::Empty => {
                game.logs(ui::EMPTY_COMMAND_MSG.into());
                game.tick();
            }
        }
    }

    println!("[ End ]");
}
