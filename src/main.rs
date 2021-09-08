#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;

mod abilities;
mod api;
mod auth;
mod body;
mod config;
mod errors;
mod expiry;
mod frontend;
mod headers;
mod models;
mod names;
mod responses;
mod schema;

use diesel::prelude::*;
use diesel::PgConnection;
use diesel_migrations::embed_migrations;

embed_migrations!();

#[database("database")]
pub struct DbConn(rocket_contrib::databases::diesel::PgConnection);

fn run_migrations(conf: &config::Config) {
    let conn =
        PgConnection::establish(&conf.make_database_url()).expect("Could not connect to database");
    embedded_migrations::run(&conn).expect("Failed to run database migrations");
}

fn main() {
    let conf = config::Config::load();
    run_migrations(&conf);
    let frontend_path = conf.frontend_path.clone();
    expiry::start_expiry_loop(&conf);
    let mut rocket = rocket::custom(conf.configure_rocket())
        .attach(DbConn::fairing())
        .attach(errors::ErrorFairing {})
        .manage(conf)
        .mount(
            "/",
            routes![
                api::create,
                api::create_without_name,
                api::get,
                api::update,
                api::delete,
                api::abilities,
                api::not_found,
                api::fallback_index,
            ],
        );
    if let Some(path) = frontend_path {
        rocket = frontend::FrontendFiles::new(path).mount(rocket);
    }
    rocket.launch();
}
