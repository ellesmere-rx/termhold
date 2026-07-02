mod game;
mod ui;

fn main() {
    println!("[ Start ]");

    let mut game = game::Game::default();

    while !game.gameover {
        ui::render(&game);
        match ui::read_command() {
            ui::CommandInput::Command(cmd) => {
                if cmd == game::Commands::Quit {
                    break;
                }
                let is_worker = cmd.is_worker_management();
                game.process_command(cmd);
                if !is_worker {
                    game.tick();
                }
            }
            ui::CommandInput::Help => {
                ui::show_help(&game);
            }
            ui::CommandInput::Invalid => {
                game.logs(ui::INVALID_COMMAND_MSG.into());
                game.tick();
            }
            ui::CommandInput::Empty => {
                game.logs(ui::EMPTY_COMMAND_MSG.into());
                game.tick();
            }
        }
    }

    println!("[ End ]");
}
