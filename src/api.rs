//! API route handlers.
use crate::abilities::Abilities;
use crate::body::Body;
use crate::config::Config;
use crate::headers::HeaderParams;
use crate::models::{Share, ShareKind};
use crate::names::{get_name, get_token};
use crate::responses::{ShareBodyResponder, ShareCreationResponder};
use crate::schema::shares;
use crate::DbConn;
use diesel::dsl::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use rocket::http::Status;
use rocket::response::status;
use rocket::State;
use rocket_contrib::json::Json;
use std::path::PathBuf;

/// Route for creating a new share.
#[post("/<name>", data = "<data>")]
pub fn create(
    conn: DbConn,
    conf: State<Config>,
    data: Body,
    name: Option<String>,
    headers: HeaderParams,
) -> Result<ShareCreationResponder, status::Custom<String>> {
    let auth = headers.get_auth(&conf)?;
    let kind = headers.get_kind()?;
    let name = get_name(&conf, &conn, &auth, name)?;
    let token = match auth.give_token() {
        true => Some(get_token()),
        false => None,
    };
    let mut share = Share::new(name.clone(), headers.get_expires(&conf), token, kind);
    match kind {
        ShareKind::Link => {
            auth.create_link()?;
            share.link = Some(data.get_link(&conf, &headers)?);
        }
        ShareKind::Paste => {
            auth.create_paste()?;
            share.language = Some(headers.get_langauage(&conf)?);
            data.write_unicode_file(&name, &conf, &headers)?;
        }
        ShareKind::File => {
            auth.create_file()?;
            share.mime_type = Some(headers.get_mime_type(&conf)?);
            data.write_raw_file(&name, &conf, &headers)?;
        }
    }
    insert_into(shares::table)
        .values(&share)
        .execute(&conn.0)
        .map_err(|_| status::Custom(Status::InternalServerError, "Database error.".into()))?;
    Ok(share.creation_response(conf))
}

#[post("/", data = "<data>")]
pub fn create_without_name(
    conn: DbConn,
    conf: State<Config>,
    data: Body,
    headers: HeaderParams,
) -> Result<ShareCreationResponder, status::Custom<String>> {
    create(conn, conf, data, None, headers)
}

/// Get a share by name.
#[get("/<name>")]
pub fn get(
    conn: DbConn,
    conf: State<Config>,
    name: String,
    headers: HeaderParams,
) -> Result<ShareBodyResponder, status::Custom<String>> {
    Ok(Share::get(name, &conn, &conf.upload_dir)?.body_response(conf, headers.accept_redirect))
}

/// Delete a share.
#[delete("/<name>")]
pub fn delete(
    conn: DbConn,
    conf: State<Config>,
    name: String,
    headers: HeaderParams,
) -> Result<status::NoContent, status::Custom<String>> {
    let share = Share::get(name, &conn, &conf.upload_dir)?;
    headers.get_auth(&conf)?.update_share(&share)?;
    if share.kind != ShareKind::Link {
        share.delete_file(&conf.upload_dir)?;
    }
    diesel::delete(shares::table.filter(shares::name.eq(share.name)))
        .execute(&conn.0)
        .map_err(|_| status::Custom(Status::InternalServerError, "Database error.".into()))?;
    Ok(status::NoContent)
}

/// Edit a share.
#[patch("/<name>", data = "<data>")]
pub fn update(
    conn: DbConn,
    conf: State<Config>,
    data: Body,
    name: String,
    headers: HeaderParams,
) -> Result<ShareBodyResponder, status::Custom<String>> {
    let mut share = Share::get(name, &conn, &conf.upload_dir)?;
    headers.get_auth(&conf)?.update_share(&share)?;
    share.expiry = headers.get_expires(&conf);
    if headers.content_length.unwrap_or(0) > 0 {
        match share.kind {
            ShareKind::Link => {
                share.link = Some(data.get_link(&conf, &headers)?);
            }
            ShareKind::Paste => {
                data.write_unicode_file(&share.name, &conf, &headers)?;
            }
            ShareKind::File => {
                data.write_raw_file(&share.name, &conf, &headers)?;
            }
        }
    }
    if headers.language.is_some() && share.kind == ShareKind::Paste {
        share.language = Some(headers.get_langauage(&conf)?);
    }
    if headers.mime_type.is_some() && share.kind == ShareKind::File {
        share.mime_type = Some(headers.get_mime_type(&conf)?);
    }
    diesel::update(shares::table.filter(shares::name.eq(share.name.clone())))
        .set(&share)
        .execute(&conn.0)
        .map_err(|_| status::Custom(Status::InternalServerError, "Database error.".into()))?;
    Ok(share.body_response(conf, headers.accept_redirect))
}

/// Get information on the features this server supports.
#[get("/meta/abilities")]
pub fn abilities(
    conf: State<Config>,
    headers: HeaderParams,
) -> Result<Json<Abilities>, status::Custom<String>> {
    let auth = headers.get_auth(&conf)?;
    Ok(Json(Abilities::load(&conf, &auth)?))
}

/// Catch-all to return a 404 error.
///
/// rank = 20 so that actual routes are still handled.
#[get("/<path..>", rank = 20)]
pub fn not_found(path: PathBuf) -> status::Custom<String> {
    status::Custom(
        Status::NotFound,
        format!("Path {} not found.", path.display()),
    )
}

/// Above catch-all won't catch the root path.
///
/// rank = 20 so that the frontend can override this if active.
#[get("/", rank = 20)]
pub fn fallback_index() -> status::Custom<String> {
    status::Custom(Status::NotFound, "Path / not found.".into())
}
