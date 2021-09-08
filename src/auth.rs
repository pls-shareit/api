//! Tools for checking client authentication and authorisation.
use crate::config::{Config, Permission, DEFAULT_PASSWORD};
use crate::models::Share;
use rocket::http::Status;
use rocket::response::status;

pub enum Auth<'a> {
    Password(&'a [Permission]),
    Default(&'a [Permission]),
    Token(String),
}

impl<'a> Auth<'a> {
    fn from_header_parts(
        conf: &'a Config,
        method: &str,
        content: &str,
    ) -> Result<Auth<'a>, status::Custom<String>> {
        match method.to_lowercase().as_str() {
            "password" => conf
                .passwords
                .get(content)
                .ok_or_else(|| {
                    status::Custom(
                        Status::Unauthorized,
                        "Given password was not recognised.".into(),
                    )
                })
                .map(|p| Auth::Password(p)),
            "token" => Ok(Auth::Token(content.to_string())),
            _ => Err(status::Custom(
                Status::BadRequest,
                "Authorization header method must be 'Password' or 'Token'.".into(),
            )),
        }
    }

    pub fn from_header(
        header: &Option<String>,
        conf: &'a Config,
    ) -> Result<Auth<'a>, status::Custom<String>> {
        match header {
            Some(header) => {
                let (method, content) = header.split_once(' ').ok_or_else(|| {
                    status::Custom(
                        Status::BadRequest,
                        "Authorization header must contain a space separated method and content."
                            .into(),
                    )
                })?;
                Auth::from_header_parts(conf, method, content)
            }
            None => Ok(Auth::Default(match conf.passwords.get(DEFAULT_PASSWORD) {
                Some(p) => p,
                None => &[],
            })),
        }
    }

    pub fn get_permissions(&self) -> Result<&[Permission], status::Custom<String>> {
        match self {
            Auth::Password(p) => Ok(p),
            Auth::Default(p) => Ok(p),
            Auth::Token(_) => Err(status::Custom(
                Status::Unauthorized,
                "Token-based authentication should not be used for this endpoint.".into(),
            )),
        }
    }

    fn assert_true(&self, value: bool, description: &str) -> Result<(), status::Custom<String>> {
        if value {
            Ok(())
        } else {
            Err(status::Custom(
                Status::Forbidden,
                format!("You do not have permission to {}.", description),
            ))
        }
    }

    fn has_permission(
        &self,
        permission: Permission,
        description: &str,
    ) -> Result<(), status::Custom<String>> {
        self.assert_true(self.get_permissions()?.contains(&permission), description)
    }

    fn can_create_share(
        &self,
        permission: Permission,
        description: &str,
    ) -> Result<(), status::Custom<String>> {
        let permissions = self.get_permissions()?;
        let allowed =
            permissions.contains(&permission) || permissions.contains(&Permission::CreateAny);
        self.assert_true(allowed, description)
    }

    pub fn create_file(&self) -> Result<(), status::Custom<String>> {
        self.can_create_share(Permission::CreateFile, "upload a file")
    }

    pub fn create_link(&self) -> Result<(), status::Custom<String>> {
        self.can_create_share(Permission::CreateLink, "create a short link")
    }

    pub fn create_paste(&self) -> Result<(), status::Custom<String>> {
        self.can_create_share(Permission::CreatePaste, "create a paste")
    }

    pub fn custom_name(&self) -> Result<(), status::Custom<String>> {
        self.has_permission(Permission::CustomName, "use a custom name")
    }

    pub fn give_token(&self) -> bool {
        let permissions = match self {
            Auth::Token(_) => return false,
            Auth::Password(permissions) => permissions,
            Auth::Default(permissions) => permissions,
        };
        permissions.contains(&Permission::UpdateOwn) || permissions.contains(&Permission::UpdateAny)
    }

    pub fn update_share(&self, share: &Share) -> Result<(), status::Custom<String>> {
        match self {
            Auth::Token(token) => {
                if Some(token) == share.token.as_ref() {
                    Ok(())
                } else {
                    Err(status::Custom(
                        Status::Unauthorized,
                        "Your share token is incorrect.".into(),
                    ))
                }
            }
            _ => self.has_permission(Permission::UpdateAny, "update shares you didn't create"),
        }
    }
}
