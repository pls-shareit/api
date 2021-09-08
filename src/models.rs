use crate::config::Config;
use crate::responses::{ShareBodyResponder, ShareCreationResponder};
use crate::schema::shares;
use crate::DbConn;
use diesel::associations::HasTable;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::Identifiable;
use diesel::{Insertable, QueryDsl, Queryable, RunQueryDsl};
use rocket::http::Status;
use rocket::response::status;
use rocket::State;
use std::convert::{TryFrom, TryInto};
use std::fs::remove_file;
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Copy, Clone, AsExpression, FromSqlRow, PartialEq, Eq)]
#[sql_type = "SmallInt"]
pub enum ShareKind {
    Link = 1,
    Paste = 2,
    File = 3,
}

impl TryFrom<i16> for ShareKind {
    type Error = String;

    fn try_from(raw: i16) -> Result<Self, Self::Error> {
        match raw {
            x if x == ShareKind::Link as i16 => Ok(ShareKind::Link),
            x if x == ShareKind::Paste as i16 => Ok(ShareKind::Paste),
            x if x == ShareKind::File as i16 => Ok(ShareKind::File),
            _ => Err("Invalid share kind.".into()),
        }
    }
}

impl<DB: Backend> ToSql<SmallInt, DB> for ShareKind
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        (*self as i16).to_sql(out)
    }
}

impl<DB: Backend> FromSql<SmallInt, DB> for ShareKind
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let kind: Result<ShareKind, _> = i16::from_sql(bytes)?.try_into();
        match kind {
            Ok(kind) => Ok(kind),
            Err(_) => Err("Invalid share kind.".into()),
        }
    }
}

#[derive(Insertable, Queryable, AsChangeset)]
pub struct Share {
    pub name: String,
    pub expiry: Option<SystemTime>,
    pub token: Option<String>,
    pub kind: ShareKind,
    pub link: Option<String>,
    pub language: Option<String>,
    pub mime_type: Option<String>,
}

impl HasTable for Share {
    type Table = shares::table;
    fn table() -> Self::Table { shares::table }
}

impl Identifiable for Share {
    type Id = String;
    fn id(self) -> Self::Id { self.name }
}

impl Share {
    pub fn new(
        name: String,
        expiry: Option<SystemTime>,
        token: Option<String>,
        kind: ShareKind,
    ) -> Self {
        Share {
            name,
            expiry,
            token,
            kind,
            link: None,
            language: None,
            mime_type: None,
        }
    }

    pub fn get(
        name: String,
        conn: &DbConn,
        upload_path: &Path,
    ) -> Result<Share, status::Custom<String>> {
        let share = shares::table
            .find(name)
            .first::<Share>(&conn.0)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    status::Custom(Status::NotFound, "Share not found.".into())
                }
                _ => status::Custom(Status::InternalServerError, "Database error.".into()),
            })?;
        if share.expiry.is_some() && share.expiry < Some(SystemTime::now()) {
            share.delete(conn, upload_path)?;
            Err(status::Custom(Status::NotFound, "Share not found.".into()))
        } else {
            Ok(share)
        }
    }

    pub fn delete_file(&self, upload_path: &Path) -> Result<(), status::Custom<String>> {
        let mut path = upload_path.to_path_buf();
        path.push(self.name.clone());
        match remove_file(&path) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => Ok(()),
                _ => Err(status::Custom(
                    Status::InternalServerError,
                    "Filesystem error.".into(),
                )),
            },
        }
    }

    pub fn delete(self, conn: &DbConn, upload_path: &Path) -> Result<(), status::Custom<String>> {
        self.delete_file(upload_path)?;
        diesel::delete(shares::table.find(self.name))
            .execute(&conn.0)
            .map_err(|_| status::Custom(Status::InternalServerError, "Database error.".into()))?;
        Ok(())
    }

    pub fn creation_response(self, conf: State<Config>) -> ShareCreationResponder {
        ShareCreationResponder {
            conf,
            name: self.name,
            token: self.token,
        }
    }

    pub fn body_response(self, conf: State<Config>) -> ShareBodyResponder {
        ShareBodyResponder {
            conf,
            name: self.name,
            kind: self.kind,
            link: self.link,
            language: self.language,
            mime_type: self.mime_type,
        }
    }
}
