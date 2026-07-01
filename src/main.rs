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
                game.process_command(cmd);
            }
            ui::CommandInput::Invalid => {
                game.logs(ui::INVALID_COMMAND_MSG.into());
            }
            ui::CommandInput::Empty => {
                game.logs(ui::EMPTY_COMMAND_MSG.into());
            }
        }
        game.tick();
    }

    println!("[ End ]");
}
