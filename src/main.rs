mod game;
mod ui;

fn main() {
    println!("[ Start ]");

    let mut game = game::Game::default();

    while !game.gameover {
        ui::render(&game);
        if let Some(cmd) = ui::read_command() {
            if cmd == game::Commands::Quit {
                break;
            }
            game.process_command(cmd);
        }
        game.tick();
    }

    println!("[ End ]");
}
