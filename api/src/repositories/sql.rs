use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::domains::Person;

use super::PeopleRepository;

#[derive(Clone)]
pub struct SqlPeopleRepository {
    pub(super) pool: PgPool,
}

impl SqlPeopleRepository {
    pub async fn connect() -> Self {
        let addr = std::env::var("PG_ADDRESS")
            .unwrap_or_else(|_| "postgres://postgres:secret@0.0.0.0:5432".into());

        let pool = PgPoolOptions::new()
            .connect(&addr)
            .await
            .expect("failed to init pool");

        Self { pool }
    }
}

#[async_trait::async_trait]
impl PeopleRepository for SqlPeopleRepository {
    async fn find_one(&self, id: Uuid) -> Result<Option<Person>> {
        sqlx::query_as(
            "\
SELECT \
    id, \
    name, \
    nickname::text, \
    birthday, \
    stack \
 FROM people \
WHERE id = $1\
",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn search_many(&self, term: &str) -> Result<Vec<Person>> {
        let term = format!("%{term}%");

        sqlx::query_as(
            "\
SELECT \
    id, \
    name, \
    nickname::text, \
    birthday, \
    stack \
 FROM people \
WHERE search_term LIKE $1 \
LIMIT 50\
",
        )
        .bind(term)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn insert_many(&self, people: &[Person]) -> Result<()> {
        if people.is_empty() {
            return Ok(());
        }

        sqlx::QueryBuilder::new("INSERT INTO people (id, name, nickname, birthday, stack)")
            .push_values(people, |mut query, person| {
                query
                    .push_bind(&person.id)
                    .push_bind(&person.name)
                    .push_bind(&person.nickname)
                    .push_bind(&person.birthday)
                    .push_bind(&person.stack);
            })
            .build()
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn count_people(&self) -> Result<i64> {
        let (rows,) = sqlx::query_as("SELECT COUNT(1) FROM people")
            .fetch_one(&self.pool)
            .await?;

        Ok(rows)
    }
}
