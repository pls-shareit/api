//! Handlers to serve frontend static files.
use rocket::http::Status;
use rocket::response::{status, NamedFile};
use rocket::{Rocket, State};
use rocket_contrib::serve::{Options, StaticFiles};
use std::path::{Path, PathBuf};

pub struct FrontendFiles {
    static_path: PathBuf,
    index_path: PathBuf,
}

impl FrontendFiles {
    fn ensure_exists(path: &Path) {
        if !path.exists() {
            panic!("Frontend path does not exist: {}", path.display());
        }
    }

    pub fn new(path: PathBuf) -> Self {
        let index_path = path.join("index.html");
        Self::ensure_exists(&index_path);
        Self {
            // We don't need to ensure the static path exists, it is optional
            // and if it does not the StaticFiles handler will return a 404.
            static_path: path.join("static"),
            index_path,
        }
    }

    pub fn mount(self, rocket: Rocket) -> Rocket {
        rocket
            .mount(
                "/static",
                StaticFiles::new(self.static_path.clone(), Options::None),
            )
            .mount("/", routes![index])
            .manage(self)
    }
}

#[get("/")]
fn index(files: State<FrontendFiles>) -> Result<NamedFile, status::Custom<String>> {
    NamedFile::open(&files.index_path).map_err(|_| {
        status::Custom(
            Status::InternalServerError,
            "Frontend index file unexpectedly missing.".into(),
        )
    })
}
