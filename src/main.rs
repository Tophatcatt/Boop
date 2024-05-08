use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::{Modifier, Span},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs, Widget},
    Frame, Terminal,
};
use rusqlite::{Connection, Result};
use std::{env, error::Error, fs, io, path::Path, process::exit, thread, time::Duration};

struct Filenames {
    name: String,
}

fn tui_table(f: &mut Frame) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(f.size())
        .to_vec()
}

fn ui(f: &mut Frame) {
    let chunks = tui_table(f);
    let block = Block::default().title("Block").borders(Borders::ALL);
    // let block = Block::default().title("Block 2").borders(Borders::ALL);
    // let block = Block::default().title("Block 3").borders(Borders::ALL);
    f.render_widget(block, chunks[1]);
}

fn tui_quit<B: Backend + std::io::Write>(term: &mut Terminal<B>) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    Ok(())
}

fn tui_draw() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    loop {
        terminal.draw(|f| {
            // let size = f.size();
            //let block = Block::default().title("Block").borders(Borders::ALL);
            //f.render_widget(block, size);
            ui(f);
        })?;
    }
    thread::sleep(Duration::from_millis(5000));

    // restore terminal
    tui_quit(&mut terminal)
}

fn open_db() -> Connection {
    let args: Vec<String> = env::args().collect(); //command line arguments
    let path: String = match env::var("HOME") {
        Ok(val) => val,
        _ => String::new(),
    };
    let filesyspath = format!("{0}/{1}", path, ".dbfs");
    fs::create_dir(&filesyspath);
    let home = Path::new(&filesyspath);
    env::set_current_dir(&home);
    let conn = Connection::open(format!("{0}/{1}", path, ".dbfs/.booper.db")).unwrap();
    conn.execute(
        //THIS IS HOW WERE GONNA DO THE DATABASE. DO IT IF IT DOESN'T EXIST
        "CREATE TABLE if not exists Files (
	id	INTEGER NOT NULL UNIQUE,
	fname	TEXT NOT NULL,
	PRIMARY KEY(id AUTOINCREMENT)
    );
    ",
        (),
    );
    conn.execute(
        "CREATE TABLE if not exists Tags (
	id	INTEGER NOT NULL UNIQUE,
	tname	TEXT NOT NULL,
	PRIMARY KEY(id AUTOINCREMENT)
    );
   ",
        (),
    );
    conn.execute(
        " CREATE TABLE if not EXISTS Tagging (
	fid	INTEGER NOT NULL,
	tid	INTEGER NOT NULL,
	FOREIGN KEY(tid) REFERENCES Tags(id),
	FOREIGN KEY(fid) REFERENCES Files(id),
	PRIMARY KEY(fid,tid)
);",
        (),
    );
    conn
}

fn selectfiles(db: &Connection, args: &Vec<String>) -> Result<()> {
    if args.len() == 2 {
        println!("No Arguments Provided");
        exit(1);
    }
    let selection = "SELECT DISTINCT fname from Files, Tagging, Tags 
        where Files.id=Tagging.fid and Tags.id=Tagging.tid 
        and Files.id in";
    let nest = "(SELECT fid from Tagging, Tags where tid=id and tname=";
    let nest2 = "and fid in (";
    let mut fin: String = String::new();
    for i in 2..args.len() - 1 {
        fin = format!("{}{}'{}') {}", fin, nest, args[i], nest2);
    }
    fin = format!("{}{}'{}')", fin, nest, args[args.len() - 1]);
    for _ in 2..args.len() - 1 {
        fin.push(')');
    }
    let selection = format!("{}{}", selection, fin);
    let mut stmt = db.prepare(selection.as_str())?;
    let fnames = stmt.query_map([], |row| Ok(Filenames { name: row.get(0)? }))?;
    for fname in fnames {
        let name = fname?.name;
        let exists = Path::new(name.as_str()).exists();
        if exists {
            println!("{}", name)
        }
    }
    Ok(())
}

fn main() {
    let db = open_db();
    //  let home = "HOME";
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("No Arguments Provided");
        return;
    }

    match args[1].as_str() {
        "-s" => {
            selectfiles(&db, &args);
        }
        _ => {
            return;
        }
    }
}
