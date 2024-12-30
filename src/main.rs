use std::env;

mod tic_tac_toe;

fn main() -> std::io::Result<()> {
    let mut args = env::args().skip(1);
    let Some(name) = args.next() else {
        println!("name of game missing");
        usage();
        std::process::exit(1);
    };
    let Some(game) = Game::from_str(&name) else {
        println!("game \"{name}\" not found");
        usage();
        std::process::exit(1);
    };
    let mut terminal = ratatui::init();
    let result = match game {
        Game::TicTacToe => tic_tac_toe::run(&mut terminal),
    };
    ratatui::restore();
    result
}

enum Game {
    TicTacToe,
}

impl Game {
    fn from_str(name: &str) -> Option<Self> {
        match name {
            "tic-tac-toe" => Some(Self::TicTacToe),
            _ => None,
        }
    }
}

fn usage() {
    println!("usage: ./games <game>");
    println!(r#"where <game> is one of ("tic-tac-toe")"#);
}
