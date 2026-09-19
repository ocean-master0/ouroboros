/// Users CRUD endpoints.
///
/// Implements AC-15 and FR11. Interacts with PostgreSQL `users` table via SQLx.
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

/// Assembly of the users sub-router.
pub fn users_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
}

#[derive(serde::Deserialize)]
pub struct UsersQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub search: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow, Clone, Debug, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct UsersListResponse {
    pub users: Vec<User>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

fn validate_user_input(name: &str, email: &str) -> Result<(), AppError> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::Validation("name cannot be empty".to_string()));
    }
    if trimmed_name.len() > 255 {
        return Err(AppError::Validation("name must not exceed 255 characters".to_string()));
    }

    let trimmed_email = email.trim();
    if trimmed_email.is_empty()
        || !trimmed_email.contains('@')
        || trimmed_email.starts_with('@')
        || trimmed_email.ends_with('@')
    {
        return Err(AppError::Validation(
            "email must contain '@' and be properly formatted".to_string(),
        ));
    }
    if trimmed_email.len() > 255 {
        return Err(AppError::Validation("email must not exceed 255 characters".to_string()));
    }

    Ok(())
}

/// GET /api/v1/users — list users with pagination and optional name/email/search filtering.
pub async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<UsersQuery>,
) -> Result<Json<UsersListResponse>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let mut builder = sqlx::QueryBuilder::new(
        "SELECT id, name, email, created_at, updated_at FROM users WHERE 1=1",
    );
    let mut count_builder = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM users WHERE 1=1");

    if let Some(ref email) = params.email {
        let pattern = format!("%{}%", email.trim());
        builder.push(" AND email ILIKE ");
        builder.push_bind(pattern.clone());
        count_builder.push(" AND email ILIKE ");
        count_builder.push_bind(pattern);
    }

    if let Some(ref name) = params.name {
        let pattern = format!("%{}%", name.trim());
        builder.push(" AND name ILIKE ");
        builder.push_bind(pattern.clone());
        count_builder.push(" AND name ILIKE ");
        count_builder.push_bind(pattern);
    }

    if let Some(ref search) = params.search {
        let pattern = format!("%{}%", search.trim());
        builder.push(" AND (name ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR email ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(")");

        count_builder.push(" AND (name ILIKE ");
        count_builder.push_bind(pattern.clone());
        count_builder.push(" OR email ILIKE ");
        count_builder.push_bind(pattern);
        count_builder.push(")");
    }

    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(page_size);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

    let users = builder
        .build_query_as::<User>()
        .fetch_all(&state.db)
        .await
        .map_err(AppError::Database)?;

    let total: (i64,) = count_builder
        .build_query_as::<(i64,)>()
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    Ok(Json(UsersListResponse {
        users,
        page,
        page_size,
        total: total.0,
    }))
}

/// POST /api/v1/users — create a new user.
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), AppError> {
    validate_user_input(&payload.name, &payload.email)?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email, created_at, updated_at",
    )
    .bind(payload.name.trim())
    .bind(payload.email.trim())
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return AppError::Conflict("email already exists".to_string());
            }
        }
        AppError::Database(e)
    })?;

    Ok((StatusCode::CREATED, Json(user)))
}

/// GET /api/v1/users/:id — retrieve a user by ID.
pub async fn get_user(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<Json<User>, AppError> {
    let id = Uuid::parse_str(&id_str)
        .map_err(|_| AppError::Validation("invalid UUID format".to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "SELECT id, name, email, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or(AppError::NotFound)?;

    Ok(Json(user))
}

/// PUT /api/v1/users/:id — update a user's details.
pub async fn update_user(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<User>, AppError> {
    let id = Uuid::parse_str(&id_str)
        .map_err(|_| AppError::Validation("invalid UUID format".to_string()))?;

    if let Some(ref name) = payload.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("name cannot be empty".to_string()));
        }
        if trimmed.len() > 255 {
            return Err(AppError::Validation("name must not exceed 255 characters".to_string()));
        }
    }

    if let Some(ref email) = payload.email {
        let trimmed = email.trim();
        if trimmed.is_empty()
            || !trimmed.contains('@')
            || trimmed.starts_with('@')
            || trimmed.ends_with('@')
        {
            return Err(AppError::Validation("email must contain '@'".to_string()));
        }
        if trimmed.len() > 255 {
            return Err(AppError::Validation("email must not exceed 255 characters".to_string()));
        }
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET 
            name = COALESCE($1, name), 
            email = COALESCE($2, email), 
            updated_at = NOW() 
         WHERE id = $3 
         RETURNING id, name, email, created_at, updated_at",
    )
    .bind(payload.name)
    .bind(payload.email)
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?
    .ok_or(AppError::NotFound)?;

    Ok(Json(user))
}

/// DELETE /api/v1/users/:id — delete a user by ID.
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Result<StatusCode, AppError> {
    let id = Uuid::parse_str(&id_str)
        .map_err(|_| AppError::Validation("invalid UUID format".to_string()))?;

    let res = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(AppError::Database)?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_validation_rules_fr11() {
        // Empty name rejected
        assert!(validate_user_input("", "test@example.com").is_err());
        assert!(validate_user_input("   ", "test@example.com").is_err());

        // Malformed email rejected
        assert!(validate_user_input("Abhishek", "bademail").is_err());
        assert!(validate_user_input("Abhishek", "@example.com").is_err());
        assert!(validate_user_input("Abhishek", "test@").is_err());
        assert!(validate_user_input("Abhishek", "").is_err());

        // Valid inputs accepted
        assert!(validate_user_input("Abhishek", "abhishek@example.com").is_ok());
    }
}
