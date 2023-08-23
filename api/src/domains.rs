use std::{collections::BTreeMap, hash::Hash};

use anyhow::Result;
use time::Date;
use uuid::Uuid;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, sqlx::FromRow)]
pub struct Person {
    pub id: Uuid,
    #[serde(alias = "name", rename = "nome")]
    pub name: String,
    #[serde(alias = "nickname", rename = "apelido")]
    pub nickname: String,
    #[serde(alias = "birthday", rename = "nascimento")]
    pub birthday: Date,
    pub stack: Option<Vec<String>>,
}

impl Person {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.nickname.len() <= 32,
            "nickname must be at max 32 chars wide"
        );
        anyhow::ensure!(self.name.len() <= 100, "name must be at max 100 chars wide");

        let all_valid = self
            .stack
            .as_ref()
            .is_some_and(|s| s.iter().all(|s| s.len() <= 32));
        anyhow::ensure!(all_valid, "stack names must be at max 32 chars wide");

        Ok(())
    }

    pub fn as_string_map(&self) -> BTreeMap<&'static str, String> {
        let mut map = BTreeMap::from([
            ("id", self.id.to_string()),
            ("name", self.name.clone()),
            ("nickname", self.nickname.clone()),
            ("birthday", self.birthday.to_string()),
        ]);

        if let Some(stack) = &self.stack {
            let stack = serde_json::to_string(&stack).unwrap();
            map.insert("stack", stack);
        }

        map
    }
}

impl Hash for Person {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Person {}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
