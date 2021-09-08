//! Tools for generating and validating share names and tokens.
use crate::auth::Auth;
use crate::config::Config;
use crate::schema::shares;
use crate::DbConn;
use diesel::dsl::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use rand::Rng;
use rocket::http::Status;
use rocket::response::status;
use std::iter::{repeat_with, Iterator};

const NAME_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const TOKEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn generate_random_string(chars: &[u8], length: usize) -> String {
    let mut rng = rand::thread_rng();
    repeat_with(|| chars[rng.gen_range(0..chars.len())])
        .take(length)
        .map(|byte| byte as char)
        .collect()
}

fn name_taken(name: &str, conn: &DbConn) -> Result<bool, status::Custom<String>> {
    select(exists(shares::table.filter(shares::name.eq(name))))
        .get_result(&conn.0)
        .map_err(|_| status::Custom(Status::InternalServerError, "Database error.".into()))
}

struct NameGenerator<'a> {
    name: Option<String>,
    attempt_count: u8,
    conf: &'a Config,
    conn: &'a DbConn,
}

impl NameGenerator<'_> {
    fn generate(conf: &Config, conn: &DbConn) -> Result<String, status::Custom<String>> {
        let generator = NameGenerator {
            name: None,
            attempt_count: 0,
            conf,
            conn,
        }
        .start()?;
        match generator.name {
            Some(name) => Ok(name),
            None => Err(status::Custom(
                Status::Conflict,
                "Could not find an unused name.".into(),
            )),
        }
    }

    fn start(mut self) -> Result<Self, status::Custom<String>> {
        loop {
            self.generate_name()?;
            if self.name.is_some() {
                break;
            }
            if self.attempt_count >= self.conf.names.random_attempt_limit {
                if self.conf.names.get_random_length()? >= self.conf.names.max_length.into() {
                    break;
                }
                self.attempt_count = 0;
                self.conf.names.incr_random_length()?;
            }
        }
        Ok(self)
    }

    fn generate_name(&mut self) -> Result<(), status::Custom<String>> {
        let name = generate_random_string(NAME_CHARS, self.conf.names.get_random_length()?);
        self.attempt_count += 1;
        self.name = if name_taken(&name, self.conn)? {
            None
        } else {
            Some(name)
        };
        Ok(())
    }
}

fn ensure_name_urlsafe(name: &str) -> Result<(), status::Custom<String>> {
    for c in name.chars() {
        if !(c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '-'
            || c == '.'
            || c == '_'
            || c == '~')
        {
            return Err(status::Custom(
                Status::BadRequest,
                format!(
                    "Invalid character {} in name, must be a-z, 0-9, _, ., ~ or -.",
                    c
                ),
            ));
        }
    }
    Ok(())
}

fn check_name_length(name: &str, conf: &Config) -> Result<(), status::Custom<String>> {
    if name.len() < conf.names.min_length.into() {
        return Err(status::Custom(
            Status::BadRequest,
            "Name is too short.".into(),
        ));
    }
    if name.len() > conf.names.max_length.into() {
        return Err(status::Custom(
            Status::BadRequest,
            "Name is too long.".into(),
        ));
    }
    Ok(())
}

fn validate_name(
    name: String,
    conf: &Config,
    conn: &DbConn,
    auth: &Auth,
) -> Result<String, status::Custom<String>> {
    auth.custom_name()?;
    ensure_name_urlsafe(&name)?;
    check_name_length(&name, conf)?;
    if name.ends_with('.') {
        return Err(status::Custom(
            Status::BadRequest,
            "Name cannot end with a period.".into(),
        ));
    }
    if name_taken(&name, conn)? {
        Err(status::Custom(
            Status::Conflict,
            "Name is already taken.".into(),
        ))
    } else {
        Ok(name)
    }
}

pub fn get_name(
    conf: &Config,
    conn: &DbConn,
    auth: &Auth,
    custom: Option<String>,
) -> Result<String, status::Custom<String>> {
    match custom {
        Some(name) => validate_name(name, conf, conn, auth),
        None => NameGenerator::generate(conf, conn),
    }
}

pub fn get_token() -> String { generate_random_string(TOKEN_CHARS, 128) }
