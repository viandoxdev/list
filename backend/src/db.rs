use serde::Serialize;
use sqlx::{
    postgres::{PgPoolOptions, PgDatabaseError},
    PgPool,
};

use crate::ListError;

const DATABASE_URL: &str = "postgres://listdbuser:listdbpass@localhost:9001/listdb";

/// Our database object used to do all operations
pub struct ListDatabase {
    pool: PgPool,
}

impl ListDatabase {
    /// Create (connect to) the database, also creates the tables if they don't exist already
    pub async fn new() -> Result<Self, ListError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(DATABASE_URL)
            .await?;

        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS lists (
                id serial PRIMARY KEY,
                name VARCHAR(255) NOT NULL UNIQUE
            );
        ",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS items (
                id serial PRIMARY KEY,
                content TEXT NOT NULL,
                list_id int NOT NULL,
                FOREIGN KEY (list_id) REFERENCES lists(id) ON DELETE CASCADE
            );
        ",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// Fetch all lists
    pub async fn fetch_lists(&self) -> Result<Vec<List>, ListError> {
        Ok(sqlx::query_as!(List, "SELECT * FROM lists")
            .fetch_all(&self.pool)
            .await?)
    }

    /// Fetch a single list
    pub async fn fetch_list(&self, list: i32) -> Result<List, ListError> {
        match sqlx::query_as!(List, "SELECT * FROM lists WHERE id = $1", list).fetch_one(&self.pool).await {
            Ok(res) => Ok(res),
            Err(sqlx::Error::RowNotFound) => Err(ListError::NoSuchList(list)),
            Err(e) => Err(ListError::SqlxError(e))
        }
    }

    /// Fetch all the items (from all lists)
    pub async fn fetch_all_items(&self) -> Result<Vec<Item>, ListError> {
        Ok(sqlx::query_as!(Item, "SELECT * FROM items")
            .fetch_all(&self.pool)
            .await?)
    }

    /// Fetch the items of a list
    pub async fn fetch_items(&self, list: i32) -> Result<Vec<Item>, ListError> {
        let res = sqlx::query!("SELECT FROM lists WHERE id = $1", list).execute(&self.pool).await?;
        (res.rows_affected() > 0).then_some(()).ok_or(ListError::NoSuchList(list))?;
        Ok(sqlx::query_as!(Item, "SELECT * FROM items WHERE list_id = $1", list).fetch_all(&self.pool).await?)
    }

    /// Fetch a single item
    pub async fn fetch_item(&self, item: i32) -> Result<Item, ListError> {
        match sqlx::query_as!(Item, "SELECT * FROM items WHERE id = $1", item).fetch_one(&self.pool).await {
            Ok(res) => Ok(res),
            Err(sqlx::Error::RowNotFound) => Err(ListError::NoSuchItem(item)),
            Err(e) => Err(ListError::SqlxError(e))
        }
    }

    /// Create a list, can only return SqlxError on failure
    pub async fn create_list(&self, name: &str) -> Result<List, ListError> {
        match sqlx::query_as!(List, "INSERT INTO lists (name) VALUES ($1) RETURNING *", name).fetch_one(&self.pool).await {
            Ok(list) => Ok(list),
            // Check if the error is a violation of unique constraint
            Err(sqlx::Error::Database(err)) if err.try_downcast_ref::<PgDatabaseError>().map(|e| e.code() == "23505").unwrap_or_default() => Err(ListError::DuplicateListName(name.into())),
            // Any other error is passed as is
            Err(e) => Err(ListError::SqlxError(e))
        }
    }

    /// Remove a list, returns either NoSuchList or SqlxError on failure
    pub async fn remove_list(&self, list: i32) -> Result<(), ListError> {
        let res = sqlx::query!("DELETE FROM lists WHERE id = $1", list).execute(&self.pool).await?;
        (res.rows_affected() > 0).then_some(()).ok_or(ListError::NoSuchList(list))
    }

    /// Rename a list, returns either NoSuchList or SqlxError on failure
    pub async fn rename_list(&self, list: i32, new_name: &str) -> Result<(), ListError> {
        let res = sqlx::query!("UPDATE lists SET name = $1 WHERE id = $2", new_name, list).execute(&self.pool).await?;
        (res.rows_affected() > 0).then_some(()).ok_or(ListError::NoSuchList(list))
    }

    /// Create an item, returns either NoSuchList or SqlxError on failure
    pub async fn create_item(&self, list: i32, content: &str) -> Result<Item, ListError> {
        match sqlx::query_as!(Item, "INSERT INTO items (list_id, content) VALUES ($1, $2) RETURNING *", list, content).fetch_one(&self.pool).await {
            Ok(item) => Ok(item),
            // Check if the error is a violation of foreign key constraint (meaning the list id is
            // wrong)
            Err(sqlx::Error::Database(err)) if err.try_downcast_ref::<PgDatabaseError>().map(|e| e.code() == "23503").unwrap_or_default() => Err(ListError::NoSuchList(list)),
            // Any other error is passed as is
            Err(e) => Err(ListError::SqlxError(e))
        }
    }

    /// Remove an item, returns either NoSuchItem or SqlxError on failure
    pub async fn remove_item(&self, item: i32) -> Result<(), ListError> {
        let res = sqlx::query!("DELETE FROM items WHERE id = $1", item).execute(&self.pool).await?;
        (res.rows_affected() > 0).then_some(()).ok_or(ListError::NoSuchItem(item))
    }

    /// Edit an item, returns either NoSuchItem or SqlxError on failure
    pub async fn edit_item(&self, item: i32, new_content: &str) -> Result<(), ListError> {
        let res = sqlx::query!("UPDATE items SET content = $1 WHERE id = $2", new_content, item).execute(&self.pool).await?;
        (res.rows_affected() > 0).then_some(()).ok_or(ListError::NoSuchItem(item))
    }
}

/// Rust representation of a list
#[derive(Debug, Clone, Serialize)]
pub struct List {
    pub id: i32,
    pub name: String,
}

/// Rust representation of an item
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: i32,
    pub list_id: i32,
    pub content: String,
}
