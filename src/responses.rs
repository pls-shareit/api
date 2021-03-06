use crate::config::Config;
use crate::models::ShareKind;
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{Responder, Response, ResponseBuilder};
use rocket::State;
use std::fs::File;
use std::io;

pub struct ShareBodyResponder<'a> {
    pub conf: State<'a, Config>,
    pub accept_redirect: bool,
    pub name: String,
    pub kind: ShareKind,
    pub link: Option<String>,
    pub language: Option<String>,
    pub mime_type: Option<String>,
}

impl<'a> ShareBodyResponder<'a> {
    fn error_response(self, response: &mut ResponseBuilder, error: String) {
        response
            .status(Status::InternalServerError)
            .header(ContentType::Plain)
            .sized_body(io::Cursor::new(error));
    }

    fn link_response(self, response: &mut ResponseBuilder) {
        match self.link {
            Some(link) => {
                let status = if self.accept_redirect {
                    Status::TemporaryRedirect
                } else {
                    Status::Ok
                };
                response
                    .status(status)
                    .raw_header("Location", link.clone())
                    .sized_body(io::Cursor::new(link));
            }
            None => self.error_response(response, "Share link unexpectedly missing.".into()),
        };
    }

    fn stream_response(self, response: &mut ResponseBuilder) -> Result<(), ()> {
        let mut path = self.conf.upload_dir.clone();
        path.push(self.name.clone());
        match File::open(path) {
            Ok(file) => {
                response.status(Status::Ok).chunked_body(file, 4096);
                Ok(())
            }
            Err(_) => {
                self.error_response(response, "Could not open file.".into());
                Err(())
            }
        }
    }

    fn paste_response(self, response: &mut ResponseBuilder) {
        match self.language.clone() {
            Some(language) => {
                if self.stream_response(response).is_ok() {
                    response
                        .raw_header("Share-Highlighting", language)
                        .header(ContentType::Plain);
                }
            }
            None => self.error_response(
                response,
                "Highlighting language unexpectedly missing.".into(),
            ),
        };
    }

    fn file_response(self, response: &mut ResponseBuilder) {
        match self.mime_type.clone() {
            Some(mime_type) => {
                if self.stream_response(response).is_ok() {
                    response.raw_header("Content-Type", mime_type);
                }
            }
            None => self.error_response(response, "Mime type unexpectedly missing.".into()),
        };
    }
}

impl<'a> Responder<'a> for ShareBodyResponder<'a> {
    fn respond_to(self, _: &Request) -> Result<Response<'a>, Status> {
        let mut response = Response::build();
        let (kind_name, _) = match self.kind {
            ShareKind::Link => ("link", self.link_response(&mut response)),
            ShareKind::Paste => ("paste", self.paste_response(&mut response)),
            ShareKind::File => ("file", self.file_response(&mut response)),
        };
        response.raw_header("Share-Type", kind_name);
        response.ok()
    }
}

pub struct ShareCreationResponder<'a> {
    pub conf: State<'a, Config>,
    pub name: String,
    pub token: Option<String>,
}

impl<'a> Responder<'a> for ShareCreationResponder<'a> {
    fn respond_to(self, _: &Request) -> Result<Response<'a>, Status> {
        let url = format!("{}{}", self.conf.network.host, self.name);
        let mut response = Response::build();
        response
            .status(Status::Created)
            .header(ContentType::Plain)
            .sized_body(io::Cursor::new(url));
        if let Some(token) = self.token {
            response.raw_header("Share-Token", token);
        };
        response.ok()
    }
}
