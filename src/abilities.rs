//! Tools for describing the features that the server supports.
use crate::auth::Auth;
use crate::config::{Config, Permission};
use rocket::response::status;
use serde::Serialize;

/// Restrictions on custom names.
#[derive(Serialize)]
pub struct NameFeatures {
    pub min_length: u8,
    pub max_length: u8,
}

#[derive(Serialize)]
pub struct Abilities {
    /// Whether you can log in to potentially get more abilities.
    pub login: bool,
    /// Whether you can create a file upload with your current password.
    pub create_file: bool,
    /// Whether you can create a paste with your current password.
    pub create_paste: bool,
    /// Whether you can create a link with your current password.
    pub create_link: bool,
    /// Whether you can update a share having created it.
    pub update_own: bool,
    /// Whether you can update any share with your current password.
    pub update_any: bool,
    /// Restrictions on custom names, or None if you cannot use custom names.
    pub custom_names: Option<NameFeatures>,
    /// The MIME types allowed for file uploads.
    pub mime_types_whitelist: Vec<String>,
    /// MIME types disallowed for file uploads. Ignored if the whitelist is not empty.
    pub mime_types_blacklist: Vec<String>,
    /// URL schemes allowed for links.
    pub link_schemes: Vec<String>,
    /// Highlighting languages allowed for pastes.
    pub highlighting_languages: Vec<String>,
}

impl Abilities {
    pub fn load(config: &Config, auth: &Auth) -> Result<Abilities, status::Custom<String>> {
        let login = !config.passwords.is_empty();
        let permissions = auth.get_permissions()?;
        let create_any = permissions.contains(&Permission::CreateAny);
        let create_file = create_any || permissions.contains(&Permission::CreateFile);
        let create_link = create_any || permissions.contains(&Permission::CreateLink);
        let create_paste = create_any || permissions.contains(&Permission::CreatePaste);
        let update_any = permissions.contains(&Permission::UpdateAny);
        let update_own = update_any || permissions.contains(&Permission::UpdateOwn);
        let custom_names = if permissions.contains(&Permission::CustomName) {
            Some(NameFeatures {
                min_length: config.names.min_length,
                max_length: config.names.max_length,
            })
        } else {
            None
        };
        let mime_types_whitelist = config.restrictions.allowed_mime_types.clone();
        let mime_types_blacklist = config.restrictions.disallowed_mime_types.clone();
        let link_schemes = config.restrictions.allowed_link_schemes.clone();
        let highlighting_languages = config.highlighting_languages.clone();
        Ok(Abilities {
            login,
            create_file,
            create_paste,
            create_link,
            update_own,
            update_any,
            custom_names,
            mime_types_whitelist,
            mime_types_blacklist,
            link_schemes,
            highlighting_languages,
        })
    }
}
