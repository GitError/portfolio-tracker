use crate::db;
use crate::error::AppError;
use crate::types::{Account, CreateAccountRequest};
use chrono::Utc;

use super::DbState;

const VALID_ACCOUNT_TYPES: &[&str] =
    &["tfsa", "rrsp", "fhsa", "taxable", "crypto", "cash", "other"];

#[tauri::command]
pub async fn get_accounts(state: tauri::State<'_, DbState>) -> Result<Vec<Account>, AppError> {
    let pool = &state.0;
    db::get_accounts(pool).await.map_err(AppError::from)
}

#[tauri::command]
pub async fn add_account(
    state: tauri::State<'_, DbState>,
    account: CreateAccountRequest,
) -> Result<Account, AppError> {
    let name = account.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation(
            "Account name cannot be empty".to_string(),
        ));
    }
    if !VALID_ACCOUNT_TYPES.contains(&account.account_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid account type: {}",
            account.account_type
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let institution = account.institution.clone();
    let account_type = account.account_type.clone();

    let pool = &state.0;
    db::insert_account(pool, &id, &name, &account_type, institution.as_deref()).await?;

    Ok(Account {
        id,
        name,
        account_type,
        institution,
        created_at,
    })
}

#[tauri::command]
pub async fn update_account(
    state: tauri::State<'_, DbState>,
    id: String,
    account: CreateAccountRequest,
) -> Result<Account, AppError> {
    let name = account.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation(
            "Account name cannot be empty".to_string(),
        ));
    }
    if !VALID_ACCOUNT_TYPES.contains(&account.account_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid account type: {}",
            account.account_type
        )));
    }

    let institution = account.institution.clone();
    let account_type = account.account_type.clone();

    let pool = &state.0;
    // Fetch created_at for the returned struct with a targeted query (avoids N+1)
    let created_at: Option<String> =
        sqlx::query_scalar("SELECT created_at FROM accounts WHERE id = $1")
            .bind(&id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)?;
    let created_at = created_at.ok_or_else(|| format!("Account {} not found", id))?;

    db::update_account(pool, &id, &name, &account_type, institution.as_deref()).await?;

    Ok(Account {
        id,
        name,
        account_type,
        institution,
        created_at,
    })
}

#[tauri::command]
pub async fn delete_account(
    state: tauri::State<'_, DbState>,
    id: String,
) -> Result<bool, AppError> {
    let pool = &state.0;
    db::delete_account(pool, &id).await?;
    Ok(true)
}
