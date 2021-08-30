//! Manages a background loop to delete expired shares.
use crate::config::Config;
use crate::models::Share;
use crate::schema::shares;
use diesel::prelude::*;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use std::path::Path;
use std::thread;

fn delete_share_files(shares: Vec<Share>, upload_path: &Path) -> Result<(), String> {
    let mut failed_deletes = vec![];
    for share in shares {
        if share.delete_file(upload_path).is_err() {
            failed_deletes.push(share.name);
        }
    }
    if failed_deletes.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Failed to delete {} shares: {}",
            failed_deletes.len(),
            failed_deletes.join(", ")
        ))
    }
}

fn clear_expired(conn: &PgConnection, upload_path: &Path) -> Result<(), String> {
    let query = shares::expiry.lt(diesel::dsl::now);
    let shares = shares::table
        .filter(query)
        .load::<Share>(conn)
        .map_err(|e| format!("Database error: {}", e))?;
    if shares.is_empty() {
        Ok(())
    } else {
        diesel::delete(shares::table.filter(query))
            .execute(conn)
            .map_err(|e| format!("Database error: {}", e))?;
        delete_share_files(shares, upload_path)
    }
}

pub fn start_expiry_loop(conf: &Config) {
    let database_url = conf.make_database_url();
    let upload_path = conf.upload_dir.clone();
    let expiry_check_interval = conf.expiry_check_interval;
    thread::spawn(move || {
        let conn = PgConnection::establish(&database_url).expect("Failed to connect to database");
        loop {
            match clear_expired(&conn, &upload_path) {
                Ok(()) => {}
                Err(e) => println!("Error clearing expired shares: {}", e),
            }
            thread::sleep(expiry_check_interval);
        }
    });
}
