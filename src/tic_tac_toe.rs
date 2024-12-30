use core::fmt::{self, Write};
use std::mem;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{
        Constraint::{self, Fill, Length},
        Layout, Rect,
    },
    style::{Color, Style},
    symbols::Marker,
    widgets::{
        block::Block,
        canvas::{Canvas, Line},
        Clear, Padding, Paragraph, Widget,
    },
    Frame,
};

pub fn run(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    let mut game = Game::new();
    loop {
        terminal.draw(|frame| game.draw(frame))?;
        if handle_events(&mut game)? {
            break Ok(());
        }
    }
}

struct Game {
    state: State,
    player_first: Player,
    score_x: usize,
    score_o: usize,
}

enum State {
    Borrowed,
    Playing(Playing),
    Done(Done),
}

impl Game {
    fn new() -> Self {
        let state = State::Playing(Playing {
            cursor_pos: 0,
            board: Board::new(),
            next: X,
        });
        Self {
            state,
            score_x: 0,
            score_o: 0,
            player_first: X,
        }
    }

    fn input_left(&mut self) {
        if let State::Playing(playing) = &mut self.state {
            playing.cursor_left();
        }
    }

    fn input_right(&mut self) {
        if let State::Playing(playing) = &mut self.state {
            playing.cursor_right();
        }
    }

    fn input_up(&mut self) {
        if let State::Playing(playing) = &mut self.state {
            playing.cursor_up();
        }
    }

    fn input_down(&mut self) {
        if let State::Playing(playing) = &mut self.state {
            playing.cursor_down();
        }
    }

    fn input_space(&mut self) {
        match self.borrow() {
            State::Playing(mut playing) => {
                playing.make_move();
                if let Some((win, player)) = playing.board.check_win() {
                    self.state = State::Done(Done {
                        board: playing.board,
                        win: Some((win, player)),
                    });
                    match player {
                        O => self.score_o += 1,
                        X => self.score_x += 1,
                    }
                } else if playing.board.is_full() {
                    self.state = State::Done(Done {
                        board: playing.board,
                        win: None,
                    })
                } else {
                    self.state = State::Playing(playing)
                }
            }
            State::Done(done) => {
                self.player_first.toggle();
                self.state = State::Playing(Playing::new(self.player_first));
            }

            State::Borrowed => unreachable!(),
        }
    }

    fn borrow(&mut self) -> State {
        mem::replace(&mut self.state, State::Borrowed)
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]);
        let [main_area, status_area] = layout.areas(frame.area());
        match &self.state {
            State::Playing(playing) => playing.draw(frame, main_area),
            State::Done(done) => done.draw(frame, main_area),
            _ => (),
        }

        // status bar
        let status_block = Block::bordered().title("Status");
        frame.render_widget(&status_block, status_area);
        let mut status = format!("Score - X: {}, O: {}", self.score_x, self.score_o);
        if let State::Playing(playing) = &self.state {
            write!(&mut status, "   {} to play", playing.next).unwrap();
        }
        let status = Paragraph::new(status);
        frame.render_widget(status, status_block.inner(status_area));
    }
}

struct Playing {
    board: Board,
    /// 0 <= cursor_pos < 9
    ///
    ///```
    /// 0 1 2
    /// 3 4 5
    /// 6 7 8
    /// ```
    cursor_pos: usize,
    next: Player,
}

impl Playing {
    fn new(first_player: Player) -> Self {
        Self {
            cursor_pos: 0,
            next: first_player,
            board: Board::new(),
        }
    }

    fn cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos / 3 * 3 + (self.cursor_pos + 2) % 3;
    }
    fn cursor_right(&mut self) {
        self.cursor_pos = self.cursor_pos / 3 * 3 + (self.cursor_pos + 1) % 3;
    }
    fn cursor_up(&mut self) {
        self.cursor_pos = (self.cursor_pos + 6) % 9;
    }
    fn cursor_down(&mut self) {
        self.cursor_pos = (self.cursor_pos + 3) % 9;
    }

    fn make_move(&mut self) {
        if self.board.squares[self.cursor_pos].is_some() {
            // square already full
            return;
        }
        self.board.squares[self.cursor_pos] = Some(self.next);
        self.next.toggle();
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        self.board.draw(Some(self.cursor_pos), frame, area);
    }
}

struct Done {
    board: Board,
    win: Option<(Win, Player)>,
}

impl Done {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        self.board.draw(None, frame, area);
        let center_vert_layout = Layout::vertical([Fill(1), Length(5), Fill(1)]);
        let center_horiz_layout = Layout::horizontal([Fill(1), Length(20), Fill(1)]);
        let text = match self.win {
            Some((_, X)) => "X won!",
            Some((_, O)) => "O won!",
            None => "draw",
        };

        // draw strikethrough
        let canvas = Canvas::default()
            .x_bounds([0., 1.])
            .y_bounds([0., 1.])
            .marker(Marker::HalfBlock)
            .paint(|ctx| {
                if let Some((win, _)) = &self.win {
                    ctx.draw(&win.line(Color::White));
                }
            });
        frame.render_widget(canvas, area);

        let para = Paragraph::new(text).centered();
        let [_, area, _] = center_vert_layout.areas(area);
        let [_, area, _] = center_horiz_layout.areas(area);
        frame.render_widget(Clear, area);

        let block = Block::bordered().padding(Padding::new(5, 5, 1, 1));
        frame.render_widget(&block, area);
        frame.render_widget(para, block.inner(area));
    }
}

enum Win {
    LeftCol,
    MidCol,
    RightCol,
    TopRow,
    MidRow,
    BottomRow,
    TLBRDiag,
    TRBLDiag,
}
use Win::*;

impl Win {
    /// Draw a line through the winning row/col/diag assuming area is (0, 0) to (1, 1)
    fn line(&self, color: Color) -> Line {
        const FIRST: f64 = 1. / 6.;
        const SECOND: f64 = 3. / 6.;
        const THIRD: f64 = 5. / 6.;
        match self {
            LeftCol => Line::new(FIRST, 0., FIRST, 1., color),
            MidCol => Line::new(SECOND, 0., SECOND, 1., color),
            RightCol => Line::new(THIRD, 0., THIRD, 1., color),
            TopRow => Line::new(0., THIRD, 1., THIRD, color),
            MidRow => Line::new(0., SECOND, 1., SECOND, color),
            BottomRow => Line::new(0., FIRST, 1., FIRST, color),
            TLBRDiag => Line::new(0., 1., 1., 0., color),
            TRBLDiag => Line::new(0., 0., 1., 1., color),
        }
    }
}

struct Board {
    squares: [Square; 9],
}

impl Board {
    fn new() -> Self {
        Self { squares: [None; 9] }
    }

    fn check_win(&self) -> Option<(Win, Player)> {
        // center first
        if let Some(player) = self.squares[4] {
            if self.squares[1] == Some(player) && self.squares[7] == Some(player) {
                return Some((MidCol, player));
            }
            if self.squares[3] == Some(player) && self.squares[5] == Some(player) {
                return Some((MidRow, player));
            }
            if self.squares[0] == Some(player) && self.squares[8] == Some(player) {
                return Some((TLBRDiag, player));
            }
            if self.squares[2] == Some(player) && self.squares[6] == Some(player) {
                return Some((TRBLDiag, player));
            }
        }
        if let Some(player) = self.squares[0] {
            if self.squares[1] == Some(player) && self.squares[2] == Some(player) {
                return Some((TopRow, player));
            }
            if self.squares[3] == Some(player) && self.squares[6] == Some(player) {
                return Some((LeftCol, player));
            }
        }
        if let Some(player) = self.squares[8] {
            if self.squares[6] == Some(player) && self.squares[7] == Some(player) {
                return Some((BottomRow, player));
            }
            if self.squares[2] == Some(player) && self.squares[5] == Some(player) {
                return Some((RightCol, player));
            }
        }
        None
    }

    fn is_full(&self) -> bool {
        self.squares.iter().all(|sq| sq.is_some())
    }

    fn draw(&self, active: Option<usize>, frame: &mut Frame<'_>, area: Rect) {
        let even_vert_layout = Layout::vertical([Fill(1), Fill(1), Fill(1)]);
        let even_horiz_layout = Layout::horizontal([Fill(1), Fill(1), Fill(1)]);
        let center_horiz_layout = Layout::horizontal([Fill(1), Length(5), Fill(1)]);
        let center_vert_layout = Layout::vertical([Fill(1), Length(5), Fill(1)]);
        let active_style = Style::new().fg(Color::Green);
        let areas: [_; 3] = even_vert_layout.areas(area);
        let block = Block::bordered();
        let area_iter = areas
            .iter()
            .map(|area| even_horiz_layout.areas::<3>(*area))
            .flatten();
        for (idx, (cell, area)) in self.squares.iter().zip(area_iter).enumerate() {
            let text = Paragraph::new(match cell {
                None => "     \n     \n     \n     \n     ",
                Some(X) => "▮   ▮\n ▮ ▮ \n  ▮  \n ▮ ▮ \n▮   ▮",
                Some(O) => " ▮▮▮ \n▮   ▮\n▮   ▮\n▮   ▮\n ▮▮▮ ",
            });
            let block = if Some(idx) == active {
                block.clone().style(active_style.clone())
            } else {
                block.clone()
            };
            let inner = block.inner(area);
            frame.render_widget(&block, area);
            let [_, c, _] = center_horiz_layout.areas(inner);
            let [_, p, _] = center_vert_layout.areas(c);
            frame.render_widget(text, p);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Player {
    X,
    O,
}
use Player::*;

impl Player {
    fn toggle(&mut self) {
        match self {
            X => *self = O,
            O => *self = X,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            X => f.write_str("X"),
            O => f.write_str("O"),
        }
    }
}

type Square = Option<Player>;

fn handle_events(game: &mut Game) -> std::io::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Left => game.input_left(),
            KeyCode::Right => game.input_right(),
            KeyCode::Up => game.input_up(),
            KeyCode::Down => game.input_down(),
            KeyCode::Char(' ') => game.input_space(),
            _ => (),
        },
        _ => (),
    }
    Ok(false)
}
