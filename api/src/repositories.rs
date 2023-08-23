pub mod sql;

use anyhow::Result;
use uuid::Uuid;

use crate::domains::Person;

#[async_trait::async_trait]
pub trait PeopleRepository {
    async fn find_one(&self, id: Uuid) -> Result<Option<Person>>;
    async fn search_many(&self, term: &str) -> Result<Vec<Person>>;
    async fn insert_many(&self, people: &[Person]) -> Result<()>;
    async fn count_people(&self) -> Result<i64>;
}
