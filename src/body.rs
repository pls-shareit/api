//! Tools for reading the request body.
use crate::config::Config;
use crate::headers::HeaderParams;
use rocket::data::{Data, DataStream, FromDataSimple, Outcome};
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::status;
use std::fs::File;
use std::io::{copy, BufReader, BufWriter, Read, Take, Write};
use unicode_reader::CodePoints;
use url::Url;

pub struct Body(Data);

impl Body {
    fn open(
        self,
        limit: u64,
        headers: &HeaderParams,
    ) -> Result<Take<DataStream>, status::Custom<String>> {
        headers.limit_content_length(limit)?;
        Ok(self.0.open().take(limit))
    }

    pub fn get_link(
        self,
        conf: &Config,
        headers: &HeaderParams,
    ) -> Result<String, status::Custom<String>> {
        let mut stream = self.open(conf.restrictions.max_link_length.into(), headers)?;
        let mut raw = String::new();
        stream.read_to_string(&mut raw).map_err(|_| {
            status::Custom(Status::BadRequest, "Could not read or decode body.".into())
        })?;
        let url = Url::parse(&raw)
            .map_err(|_| status::Custom(Status::BadRequest, "Invalid URL.".into()))?;
        let link_schemes = &conf.restrictions.allowed_link_schemes;
        if !link_schemes.is_empty() && !link_schemes.contains(&url.scheme().to_string()) {
            return Err(status::Custom(
                Status::BadRequest,
                "Invalid URL scheme.".into(),
            ));
        }
        Ok(url.as_str().to_string())
    }

    fn get_in_stream(
        self,
        conf: &Config,
        headers: &HeaderParams,
    ) -> Result<Take<DataStream>, status::Custom<String>> {
        self.open(conf.restrictions.max_upload_size.get_bytes(), headers)
    }

    fn get_out_stream(
        &self,
        name: &str,
        conf: &Config,
    ) -> Result<BufWriter<File>, status::Custom<String>> {
        let mut path = conf.upload_dir.clone();
        path.push(name);
        let file = File::create(path).map_err(|_| {
            status::Custom(Status::InternalServerError, "Could not open file.".into())
        })?;
        Ok(BufWriter::new(file))
    }

    pub fn write_raw_file(
        self,
        name: &str,
        conf: &Config,
        headers: &HeaderParams,
    ) -> Result<(), status::Custom<String>> {
        let mut out_stream = self.get_out_stream(name, conf)?;
        let mut in_stream = self.get_in_stream(conf, headers)?;
        copy(&mut in_stream, &mut out_stream).map_err(|_| {
            status::Custom(Status::InternalServerError, "Could not write file.".into())
        })?;
        Ok(())
    }

    pub fn write_unicode_file(
        self,
        name: &str,
        conf: &Config,
        headers: &HeaderParams,
    ) -> Result<(), status::Custom<String>> {
        let mut out_stream = self.get_out_stream(name, conf)?;
        let code_points = CodePoints::from(BufReader::new(self.get_in_stream(conf, headers)?));
        for code_point in code_points {
            match code_point {
                Ok(c) => {
                    out_stream
                        .write(c.encode_utf8(&mut [0; 4]).as_bytes())
                        .map_err(|_| {
                            status::Custom(
                                Status::InternalServerError,
                                "Could not write file.".into(),
                            )
                        })?;
                }
                Err(_) => {
                    return Err(status::Custom(
                        Status::BadRequest,
                        "Could not decode body.".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl FromDataSimple for Body {
    type Error = String;

    fn from_data(_request: &Request, data: Data) -> Outcome<Self, String> {
        Outcome::Success(Body(data))
    }
}
