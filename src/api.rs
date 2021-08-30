//! API route handlers.
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
use std::path::PathBuf;

/// Route for creating a new share.
///
/// Must have the Share-Type header.
///
/// - If the Share-Type header is "link", the body must be a URL.
/// - If the Share-Type header is "paste", the body must be text.
///   The Share-Highlighting may be the name of a supported language
///   for syntax highlighting.
/// - If the Share-Type header is "file", the body must be a file.
///   The Content-Type header should be present, but will default to
///   "application/octet-stream".
///
/// If the Expire-After header is present, the share will expire after
/// the given number of seconds.
///
/// If a name is not given, a random name will be generated. Returns a 201 if
/// successful, containing a link to the new share. The Share-Token header will
/// be set to the token of the new share, which can be used to edit or delete
/// the share.
#[post("/<name>", data = "<data>")]
pub fn create(
    conn: DbConn,
    conf: State<Config>,
    data: Body,
    name: Option<String>,
    headers: HeaderParams,
) -> Result<ShareCreationResponder, status::Custom<String>> {
    headers.check_password(&conf)?;
    let kind = headers.get_kind()?;
    let name = get_name(&conf, &conn, name)?;
    let mut share = Share::new(name.clone(), headers.get_expires(&conf), get_token(), kind);
    match kind {
        ShareKind::Link => {
            share.link = Some(data.get_link(&conf, &headers)?);
        }
        ShareKind::Paste => {
            share.language = Some(headers.get_langauage(&conf)?);
            data.write_unicode_file(&name, &conf, &headers)?;
        }
        ShareKind::File => {
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
///
/// Returns a redirect to the share if it is a link, otherwise the share
/// content, with the appropriate Content-Type header for files and
/// Share-Highlighting for pastes.
#[get("/<name>")]
pub fn get(
    conn: DbConn,
    conf: State<Config>,
    name: String,
) -> Result<ShareBodyResponder, status::Custom<String>> {
    Ok(Share::get(name, &conn, &conf.upload_dir)?.body_response(conf))
}

/// Delete a share.
///
/// The Authorization header must include the share token. Returns a 204 if
/// successful.
#[delete("/<name>")]
pub fn delete(
    conn: DbConn,
    conf: State<Config>,
    name: String,
    headers: HeaderParams,
) -> Result<status::NoContent, status::Custom<String>> {
    if !conf.restrictions.allow_updates {
        return Err(status::Custom(
            Status::Forbidden,
            "Deleting a share is not allowed.".into(),
        ));
    }
    let share = Share::get(name, &conn, &conf.upload_dir)?;
    headers.check_token(&share.token)?;
    if share.kind != ShareKind::Link {
        share.delete_file(&conf.upload_dir)?;
    }
    diesel::delete(shares::table.filter(shares::name.eq(share.name)))
        .execute(&conn.0)
        .map_err(|_| status::Custom(Status::InternalServerError, "Database error.".into()))?;
    Ok(status::NoContent)
}

/// Edit a share.
///
/// The Authorization header must include the share token. For files, the
/// Content-Type header may be set to change the file type. For pastes, the
/// Share-Highlighting header may be set to change the syntax highlighting
/// language.
///
/// The Expire-After header will be treated the same as when creating a share:
/// if it is not present, the share expiry will be set to the maximum (or not
/// to expire, if there is no maximum).
///
/// A body may be present to replace the existing share.
/// It is not possible to change the type or name of a share.
#[patch("/<name>", data = "<data>")]
pub fn update(
    conn: DbConn,
    conf: State<Config>,
    data: Body,
    name: String,
    headers: HeaderParams,
) -> Result<ShareBodyResponder, status::Custom<String>> {
    if !conf.restrictions.allow_updates {
        return Err(status::Custom(
            Status::Forbidden,
            "Updating a share is not allowed.".into(),
        ));
    }
    let mut share = Share::get(name, &conn, &conf.upload_dir)?;
    headers.check_token(&share.token)?;
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
    Ok(share.body_response(conf))
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
