use crate::{auth::AuthUser, error::{AppError, AppResult}, models::Role};

pub fn require_roles(auth: &AuthUser, roles: &[Role]) -> AppResult<()> {
    if roles.iter().any(|r| r == &auth.role) {
        Ok(())
    } else {
        Err(AppError::forbidden("insufficient role"))
    }
}

pub fn require_admin(auth: &AuthUser) -> AppResult<()> {
    require_roles(auth, &[Role::Admin])
}

pub fn require_user_or_admin(auth: &AuthUser) -> AppResult<()> {
    require_roles(auth, &[Role::User, Role::Admin])
}

pub fn require_user_admin_or_auditor(auth: &AuthUser) -> AppResult<()> {
    require_roles(auth, &[Role::User, Role::Admin, Role::Auditor])
}

pub fn require_admin_or_auditor(auth: &AuthUser) -> AppResult<()> {
    require_roles(auth, &[Role::Admin, Role::Auditor])
}
