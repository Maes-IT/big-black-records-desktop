use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

const SCHEMA: &str = include_str!("../migrations/schema.sql");

pub fn init_db(app_data_dir: &Path) -> Connection {
    std::fs::create_dir_all(app_data_dir).expect("cannot create app data dir");
    let db_path = app_data_dir.join("bigblack.sqlite");
    let conn = Connection::open(&db_path).expect("cannot open database");
    conn.execute_batch(SCHEMA).expect("cannot initialize schema");
    crate::seed::seed_if_empty(&conn).expect("cannot seed database");
    conn
}
