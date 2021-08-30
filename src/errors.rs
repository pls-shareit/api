//! A fairing which logs internal server errors.
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Status;

pub struct ErrorFairing;

impl Fairing for ErrorFairing {
    fn info(&self) -> Info {
        Info {
            name: "Internal Error Logging",
            kind: Kind::Response,
        }
    }

    fn on_response(&self, request: &rocket::Request, response: &mut rocket::Response) {
        if response.status() == Status::InternalServerError {
            eprintln!(
                "Internal server error!\n  {method} {uri}\n  {body:?}",
                method = request.method(),
                uri = request.uri(),
                body = response.body(),
            );
        }
    }
}
