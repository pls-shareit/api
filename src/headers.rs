//! Tools for parsing HTTP headers.
use crate::auth::Auth;
use crate::config::Config;
use crate::models::ShareKind;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::status;
use rocket::State;
use std::time::{Duration, SystemTime};

pub struct HeaderParams {
    raw_auth: Option<String>,
    kind: Option<ShareKind>,
    pub language: Option<String>,
    pub mime_type: Option<String>,
    expire_after: Option<Duration>,
    pub content_length: Option<u64>,
    pub accept_redirect: bool,
}

impl HeaderParams {
    fn parse_kind(raw: Option<&str>) -> Result<Option<ShareKind>, (Status, String)> {
        let kind_or_error = raw.map(|s| match s {
            "link" => Ok(ShareKind::Link),
            "paste" => Ok(ShareKind::Paste),
            "file" => Ok(ShareKind::File),
            _ => Err((
                Status::BadRequest,
                "Share-Type must be link, paste or file.".into(),
            )),
        });
        match kind_or_error {
            Some(Ok(kind)) => Ok(Some(kind)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    fn parse_expire_after(raw: Option<&str>) -> Result<Option<Duration>, (Status, String)> {
        match raw {
            Some(as_string) => {
                let seconds: u64 = as_string.parse().map_err(|_| {
                    (
                        Status::BadRequest,
                        "Expire-After must be an integer.".into(),
                    )
                })?;
                Ok(Some(Duration::from_secs(seconds)))
            }
            None => Ok(None),
        }
    }

    fn parse_content_length(raw: Option<&str>) -> Option<u64> {
        match raw {
            Some(as_string) => match as_string.parse::<u64>() {
                Ok(content_length) => Some(content_length),
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn get_expires(&self, conf: &State<Config>) -> Option<SystemTime> {
        match (self.expire_after, conf.restrictions.max_expiry_time) {
            (Some(expiry), Some(max_expiry)) => {
                if expiry > max_expiry {
                    Some(SystemTime::now() + max_expiry)
                } else {
                    Some(SystemTime::now() + expiry)
                }
            }
            (Some(expiry), None) => Some(SystemTime::now() + expiry),
            (None, Some(max_expiry)) => Some(SystemTime::now() + max_expiry),
            (None, None) => None,
        }
    }

    pub fn get_langauage(&self, conf: &State<Config>) -> Result<String, status::Custom<String>> {
        match &self.language {
            Some(lang) => {
                if conf.highlighting_languages.contains(lang) {
                    Ok(lang.to_string())
                } else {
                    Err(status::Custom(
                        Status::BadRequest,
                        "Given Share-Highlighting is not supported.".into(),
                    ))
                }
            }
            None => Ok(conf.default_highlighting_language.clone()),
        }
    }

    fn mime_type_allowed(&self, conf: &State<Config>, mime_type: &str) -> bool {
        let mime_type = mime_type.to_string();
        if !conf.restrictions.allowed_mime_types.is_empty() {
            conf.restrictions.allowed_mime_types.contains(&mime_type)
        } else {
            !conf.restrictions.disallowed_mime_types.contains(&mime_type)
        }
    }

    pub fn get_mime_type(&self, conf: &State<Config>) -> Result<String, status::Custom<String>> {
        match &self.mime_type {
            Some(mime_type) => {
                if self.mime_type_allowed(conf, mime_type) {
                    Ok(mime_type.into())
                } else {
                    Err(status::Custom(
                        Status::Forbidden,
                        "Given Content-Type is not allowed.".into(),
                    ))
                }
            }
            None => Ok(conf.default_mime_type.clone()),
        }
    }

    pub fn get_kind(&self) -> Result<ShareKind, status::Custom<String>> {
        match self.kind {
            Some(kind) => Ok(kind),
            None => Err(status::Custom(
                Status::BadRequest,
                "Share-Type is required.".into(),
            )),
        }
    }

    /// Ensure that the Content-Length header is lower than a given value.
    ///
    /// Note that we are not blindly trusting the header, we will still
    /// truncate the stream with the same maximum length, this is just so we
    /// can give a useful error message.
    pub fn limit_content_length(&self, limit: u64) -> Result<(), status::Custom<String>> {
        match self.content_length {
            Some(content_length) => {
                if content_length > limit {
                    Err(status::Custom(
                        Status::BadRequest,
                        "Body is too large.".into(),
                    ))
                } else {
                    Ok(())
                }
            }
            None => Ok(()),
        }
    }

    pub fn get_auth<'a>(
        &self,
        conf: &'a State<Config>,
    ) -> Result<Auth<'a>, status::Custom<String>> {
        Auth::from_header(&self.raw_auth, conf)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for HeaderParams {
    type Error = String;

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let headers = request.headers();
        let language = headers.get_one("Share-Highlighting").map(|s| s.to_string());
        let mime_type = headers.get_one("Content-Type").map(|s| s.to_string());
        let raw_auth = headers.get_one("Authorization").map(|s| s.to_string());
        let accept_redirect = !matches!(headers.get_one("Accept-Redirect"), Some("no"));
        let kind = match Self::parse_kind(headers.get_one("Share-Type")) {
            Ok(kind) => kind,
            Err(e) => return Outcome::Failure(e),
        };
        let expire_after = match Self::parse_expire_after(headers.get_one("Expire-After")) {
            Ok(expires) => expires,
            Err(e) => return Outcome::Failure(e),
        };
        let content_length = Self::parse_content_length(headers.get_one("Content-Length"));
        Outcome::Success(HeaderParams {
            raw_auth,
            kind,
            language,
            mime_type,
            expire_after,
            content_length,
            accept_redirect,
        })
    }
}
