use open;
use rusqlite::{Connection, Result};
use std::{
    env, error::Error, ffi::OsStr, fs, io, path::Path, process::exit, thread, time::Duration,
};
struct Filenames {
    name: String,
}
struct Tagging {
    fname: String,
    tname: String,
}
fn get_path() -> String {
    let home = env::var("HOME").unwrap();
    let basepath: String = format!("{}/.dbfs", home);
    return basepath;
}
fn open_db() -> Connection {
    let args: Vec<String> = env::args().collect(); //command line arguments
    let path: String = match env::var("HOME") {
        Ok(val) => val,
        _ => String::new(),
    };
    let filesyspath = format!("{0}/{1}", path, ".dbfs");
    fs::create_dir(&filesyspath);
    let conn = Connection::open(format!("{0}/{1}", path, ".dbfs/.booper.db")).unwrap();
    conn.execute(
        //THIS IS HOW WERE GONNA DO THE DATABASE. DO IT IF IT DOESN'T EXIST
        "CREATE TABLE if not exists Files (
	id	INTEGER NOT NULL UNIQUE,
	fname	TEXT NOT NULL UNIQUE,
	PRIMARY KEY(id AUTOINCREMENT)
    );
    ",
        (),
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE if not exists Tags (
	id	INTEGER NOT NULL UNIQUE,
	tname	TEXT NOT NULL UNIQUE,
	PRIMARY KEY(id AUTOINCREMENT)
    );
   ",
        (),
    )
    .unwrap();
    conn.execute(
        " CREATE TABLE if not EXISTS Tagging (
	fid	INTEGER NOT NULL,
	tid	INTEGER NOT NULL,
	FOREIGN KEY(tid) REFERENCES Tags(id),
	FOREIGN KEY(fid) REFERENCES Files(id),
	PRIMARY KEY(fid,tid)
);",
        (),
    )
    .unwrap();
    conn
}

fn file_exists(fname: &String) -> bool {
    return Path::new(format!("{}/{}", get_path(), fname).as_str()).exists();
}

fn select_tags(db: &Connection) -> Result<()> {
    let sql = "SELECT tname FROM tags";
    let mut stmt = db.prepare(sql)?;
    let tnames = stmt.query_map([], |row| Ok(Filenames { name: row.get(0)? }))?;
    for tname in tnames {
        let name = tname?.name;
        println!("{}", name)
    }
    return Ok(());
}

fn selectfiles(db: &Connection, args: &Vec<String>) -> Result<()> {
    if args.len() == 2 {
        let sql = "SELECT fname FROM Files";
        let mut stmt = db.prepare(sql)?;
        let fnames = stmt.query_map([], |row| Ok(Filenames { name: row.get(0)? }))?;
        for fname in fnames {
            let name = fname?.name;
            let exists = Path::new(format!("{}/{}", get_path(), name).as_str()).exists();
            if exists {
                println!("{}", name)
            } else {
                del_entry(&name, db);
            }
        }
        return Ok(());
    }
    let mut selection =
        "SELECT DISTINCT fname from Files join Tagging on id=fid WHERE fid".to_string();
    let nest="(SELECT fid from Tagging join Tags on id=tid WHERE tid in (SELECT id from Tags WHERE tname=";

    let mut operator: &str;
    for i in 2..args.len() - 1 {
        let arg = &args[i].trim_start_matches("-").to_string();
        if args[i].starts_with("-") {
            operator = "not in";
        } else {
            operator = "in";
        }
        selection = format!("{} {}{}'{}')) and fid ", selection, operator, nest, arg);
    }
    let len = args.len() - 1;
    let arg = &args[len].trim_start_matches("-").to_string();
    if args[len].starts_with("-") {
        operator = "not in";
    } else {
        operator = "in";
    }
    selection = format!("{} {}{}'{}'))", selection, operator, nest, arg);
    let mut stmt = db.prepare(selection.as_str())?;
    let fnames = stmt.query_map([], |row| Ok(Filenames { name: row.get(0)? }))?;
    for fname in fnames {
        let name = fname?.name;
        let exists = Path::new(format!("{}/{}", get_path(), name).as_str()).exists();
        if exists {
            println!("{}", name)
        } else {
            del_entry(&name, db);
        }
    }
    Ok(())
}

fn del_entry(fname: &String, db: &Connection) -> Result<()> {
    let file_exists = db
        .prepare(format!("SELECT * FROM Files WHERE fname='{}'", fname).as_str())?
        .exists(())?;
    if !file_exists {
        println!("File {} Does Not Exist In Database", fname);
        return Ok(());
    }
    let fsql = format!("DELETE FROM Files WHERE fname='{}'", fname);
    let tisql = format!(
        "DELETE FROM Tagging WHERE fid=(SELECT id FROM Files WHERE fname='{}')",
        fname
    );
    db.execute(tisql.as_str(), ());
    db.execute(fsql.as_str(), ());
    Ok(())
}

fn del_file(args: &Vec<String>, db: &Connection) -> Result<()> {
    let path = get_path();
    for i in 2..args.len() {
        let full_path = format!("{}/{}", path, args[i]);
        if Path::new(full_path.as_str()).exists() {
            fs::remove_file(full_path);
            del_entry(&args[i], db);
            println!("File {} Deleted", args[i]);
        } else {
            println!("File {} Does Not Exist", args[i]);
        }
    }
    Ok(())
}

fn create_file(args: &Vec<String>, db: &Connection) {
    let filepath = format!("{}/{}", get_path(), args[2]);
    if Path::new(&filepath).exists() {
        println!("Filename Already Exists");
        return;
    }
    fs::write(filepath, "");
    db.execute(
        format!("INSERT INTO Files(fname) VALUES ('{}')", args[2]).as_str(),
        (),
    )
    .unwrap();
    for i in 3..args.len() {
        create_tags(&args[i], db);
        bind_tag(&args[i], &args[2], db);
    }
}

fn create_tags(tag: &String, db: &Connection) -> Result<()> {
    let tag_exists = db
        .prepare(format!("SELECT * FROM Tags WHERE tname='{}'", tag).as_str())?
        .exists(())?;
    if !tag_exists {
        let sql = format!("INSERT INTO Tags(tname) VALUES ('{}')", tag);
        db.execute(sql.as_str(), ()).unwrap();
    } else {
        println!("Tag {} Already Exists", tag);
    }
    Ok(())
}
fn bind_tag(tag: &String, fname: &String, db: &Connection) {
    let sql = format!(
        "INSERT INTO Tagging(fid, tid)
        VALUES ((SELECT id FROM Files WHERE fname='{}'), (SELECT id FROM Tags WHERE tname='{}'))
        ",
        fname, tag
    );
    db.execute(sql.as_str(), ());
}
fn unbind_tag(file: &String, tag: &String, db: &Connection) {
    let sql=format!("DELETE FROM Tagging WHERE fid=(SELECT id from Files WHERE fname='{}') and tid=(SELECT id from Tags WHERE tname='{}')", file, tag);
    db.execute(sql.as_str(), ());
}
fn error_exit(error: &str) {
    println!("{}", error);
    exit(1);
}
fn copy_in(args: &Vec<String>, db: &Connection) {
    let file = &args[2];
    let path = std::path::Path::new(file);
    let filename = path
        .file_name()
        .unwrap_or(OsStr::new(""))
        .to_str()
        .unwrap_or("");
    if filename == "" {
        eprintln!("Invalid File Path");
        exit(1);
    }
    let file_path = format!("{}/{}", get_path(), filename);
    fs::write(&file_path, "").unwrap_or_else(|e| error_exit("File Creation Error"));
    fs::copy(file, file_path);
    let sql = format!("INSERT INTO Files(fname) VALUES ('{}')", filename);
    db.execute(sql.as_str(), ()).unwrap();
    for i in 3..args.len() {
        create_tags(&args[i], db);
        bind_tag(&args[i], &filename.to_string(), db);
    }
}
fn copy_out(file: &String, db: &Connection) {
    if !file_exists(file) {
        del_file(&vec![file.clone()], db);
        println!("File {} Does Not Exist", file);
    }
    let path = std::path::Path::new(file);
    let filename = path
        .file_name()
        .unwrap_or(OsStr::new(""))
        .to_str()
        .unwrap_or("");
    if filename == "" {
        eprintln!("Invalid File Path");
        exit(1);
    }
    let file_path = format!("{}/{}", get_path(), filename);
    fs::write(file, "").unwrap_or_else(|e| error_exit("File Creation Error"));
    fs::copy(file_path, file);
}
fn del_tag(tag: &String, db: &Connection) {
    let sql = format!(
        "DELETE FROM Tagging WHERE tid=(SELECT id FROM Tags WHERE tname='{}')",
        tag
    );
    let sql2 = format!("DELETE FROM Tags WHERE tname='{}'", tag);
    db.execute(sql.as_str(), ());
    db.execute(sql2.as_str(), ());
}
fn show_with_tags(db: &Connection) -> Result<()> {
    let sql="SELECT fname, tname from Tags, Files, Tagging WHERE fid=Files.id and tid=Tags.id ORDER BY fname";
    let mut stmt = db.prepare(sql)?;
    let taggers = stmt.query_map([], |row| {
        Ok(Tagging {
            fname: row.get(0)?,
            tname: row.get(1)?,
        })
    })?;
    let mut name = String::new();
    for duo in taggers {
        let d = duo?;
        if name != d.fname {
            name = d.fname;
            let exists = Path::new(format!("{}/{}", get_path(), name).as_str()).exists();
            if exists {
                println!();
                print!("{}: ", name);
                print!("{}", d.tname);
            } else {
                del_entry(&name, db);
            }
        } else {
            print!(", {}", d.tname);
        }
    }
    println!();
    Ok(())
}
fn commands() {
    println!("show: show all files if no arguments provided, otherwise show files with or without provided tags");
    println!("tags: show all tags");
    println!("tagging: show all files with their tags");
    println!("create: create a new file. first argument being the name and all other arguments being tags");
    println!("del: delete files. all argument being the file names");
    println!("deltag: delete tags. all arguments being the tag names");
    println!("bind: bind tags to a file. first argument being the filename, all other arguments being tags");
    println!("unbind: unbind tags from a file. first argument being the filename, all other arguments being tags");
    println!("copyin: copy a file to the Boop filesystem. first argument being the file path");
    println!("copyout: copy a file from the Boop filesystem. first argument being the file name");
    println!("open: open a file using its default program. first argument being the file name");
    println!("create_tag: create new tags from provided arguments");
    println!("help: show this list");
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
        "show" => {
            selectfiles(&db, &args);
        }
        "create" => {
            create_file(&args, &db);
        }
        "del" => {
            del_file(&args, &db);
        }
        "bind" => {
            for i in 3..args.len() {
                create_tags(&args[i], &db);
                bind_tag(&args[i], &args[2], &db);
            }
        }
        "unbind" => {
            for i in 3..args.len() {
                unbind_tag(&args[2], &args[i], &db);
            }
        }
        "copyin" => {
            copy_in(&args, &db);
        }
        "copyout" => {
            copy_out(&args[2], &db);
        }
        "open" => {
            open::that(format!("{}/{}", get_path(), args[2]))
                .unwrap_or_else(|_| error_exit("Open Failed"));
        }
        "tags" => {
            select_tags(&db);
        }
        "deltag" => {
            for i in 2..args.len() {
                del_tag(&args[i], &db)
            }
        }
        "tagging" => {
            show_with_tags(&db);
        }
        "create_tag" => {
            for i in 2..args.len() {
                create_tags(&args[i], &db);
            }
        }
        "help" => {
            commands();
        }
        _ => {
            commands();
            return;
        }
    }
}
