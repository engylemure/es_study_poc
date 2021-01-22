use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub age: u8,
    pub job: Option<String>,
    pub relationship_status: Option<RelationshipStatus>,
}

impl User {
    pub fn from_input(
        UserInput {
            name,
            age,
            job,
            relationship_status,
        }: UserInput,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            age,
            job,
            relationship_status,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInput {
    pub name: String,
    pub age: u8,
    pub job: Option<String>,
    pub relationship_status: Option<RelationshipStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipStatus {
    Single,
    Married,
}

pub struct ElasticSearchClient {
    address: String,
    client: reqwest::Client,
}

#[derive(Debug)]
pub enum ESError {
    ConnectionError,
    DeserializationError,
    NotFoundError,
    InvalidAddressError,
}

impl ElasticSearchClient {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            address: format!("http://{}:{}", host, port),
            client: reqwest::Client::new(),
        }
    }

    fn entity_url(&self, index: &str, id: &str) -> String {
        format!("{}/{}/_doc/{}", self.address, index, id)
    }

    fn search_url_with_query(&self, query: &str) -> String {
        format!("{}/_search?q={}", self.address, query)
    }
    fn search_url(&self) -> String {
        format!("{}/_search", self.address)
    }

    pub async fn post<T>(
        &self,
        index: &str,
        id: &str,
        data: &T,
    ) -> Result<ESActionInfo<()>, ESError>
    where
        T: Serialize + DeserializeOwned,
    {
        match self
            .client
            .post(&self.entity_url(index, id))
            .json(data)
            .send()
            .await
        {
            Ok(resp) => Self::resp_into_type::<ESActionInfo<()>>(resp).await,
            _ => Err(ESError::ConnectionError),
        }
    }

    pub async fn get<T>(&self, index: &str, id: &str) -> Result<ESActionInfo<T>, ESError>
    where
        T: Serialize + DeserializeOwned,
    {
        match self.client.get(&self.entity_url(index, id)).send().await {
            Ok(resp) => {
                let action = Self::resp_into_type::<ESActionInfo<T>>(resp).await?;
                Ok(action)
            }
            _ => Err(ESError::ConnectionError),
        }
    }

    pub async fn search<T>(&self, input: &SearchInput) -> Result<ESSearchResult, ESError>
    where
        T: Serialize + DeserializeOwned,
    {
        // dbg!(serde_json::to_value(input));
        match &input.query {
            QueryInput::Text(query) => {
                match self
                    .client
                    .get(&self.search_url_with_query(query))
                    .json(input)
                    .send()
                    .await
                {
                    Ok(resp) => Ok(Self::resp_into_type::<ESSearchResult>(resp).await?),
                    _ => Err(ESError::ConnectionError),
                }
            }
            _ => match self
                .client
                .post(&self.search_url())
                .json(input)
                .send()
                .await
            {
                Ok(resp) => Ok(Self::resp_into_type::<ESSearchResult>(resp).await?),
                _ => Err(ESError::ConnectionError),
            },
        }
    }

    async fn resp_into_type<T>(resp: Response) -> Result<T, ESError>
    where
        T: DeserializeOwned,
    {
        let json = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|_| ESError::DeserializationError)?;
        // dbg!(&json);
        serde_json::from_value::<T>(json).map_err(|_| ESError::DeserializationError)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ESActionInfo<T> {
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_type")]
    pub _type: String,
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_version")]
    pub version: u64,
    pub result: Option<ESActionResult>,
    pub created: Option<bool>,
    pub found: Option<bool>,
    #[serde(rename = "_source")]
    pub source: Option<T>,
}
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum ESActionResult {
    #[serde(rename = "created")]
    Created,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ESSearchResult {
    pub took: u64,
    pub timed_out: bool,
    pub hits: ESSearchResultHits,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ESSearchResultHits {
    pub total: serde_json::Value,
    pub max_score: Option<f32>,
    pub hits: Vec<ESSearchResultHit>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct ESSearchResultHit {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_score")]
    pub score: f32,
    #[serde(rename = "_source")]
    pub source: serde_json::Value,
}

impl<T> ESActionInfo<T> {
    pub fn created(&self) -> bool {
        self.created.unwrap_or(false)
    }

    pub fn is_result_type(&self, result: ESActionResult) -> bool {
        self.result
            .as_ref()
            .map(|res| res == &result)
            .unwrap_or(false)
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct SearchInput {
    from: Option<u64>,
    size: Option<u64>,
    query: QueryInput,
}

impl SearchInput {
    pub fn new(query: QueryInput, size: Option<u64>, from: Option<u64>) -> Self {
        Self { query, from, size }
    }
}

#[derive(Debug, PartialEq)]
pub enum QueryInput {
    Text(String),
    Bool(QueryDSLInput),
    MatchAll,
}

impl Serialize for QueryInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            QueryInput::Text(_) => serializer.serialize_none(),
            QueryInput::Bool(input) => {
                let mut state = serializer.serialize_struct("QueryInput::Bool", 1)?;
                state.serialize_field("bool", input)?;
                state.end()
            }
            QueryInput::MatchAll => {
                let mut state = serializer.serialize_struct("QueryInput::Bool", 1)?;
                state.serialize_field("match_all", &serde_json::json!({}))?;
                state.end()
            }
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct QueryDSLInput {
    pub must: Option<Vec<MatchClause>>,
    pub must_not: Option<Vec<MatchClause>>,
    pub filter: Option<Vec<FilterClause>>,
    pub should: Option<Vec<TermClause>>,
}

impl Serialize for QueryDSLInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("QueryDSLInput", 4)?;
        if let Some(must) = self.must.as_ref() {
            if must.len() > 0 {
                state.serialize_field("must", must)?;
            }
        }
        if let Some(must_not) = self.must_not.as_ref() {
            if must_not.len() > 0 {
                state.serialize_field("must_not", must_not)?;
            }
        }
        if let Some(filter) = self.filter.as_ref() {
            if filter.len() > 0 {
                state.serialize_field("filter", filter)?;
            }
        }
        if let Some(should) = self.should.as_ref() {
            if should.len() > 0 {
                state.serialize_field("should", should)?;
            }
        }
        state.end()
    }
}

#[derive(Debug, PartialEq)]
pub struct MatchClause {
    name: String,
    search: String,
}

impl MatchClause {
    pub fn new(name: String, search: String) -> Self {
        Self { name, search }
    }
}

impl Serialize for MatchClause {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("MatchClause", 1)?;
        let MatchClause { name, search } = self;
        let data = serde_json::json!({ name: search });
        state.serialize_field("match", &data)?;
        state.end()
    }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum FilterClause {
    Term(TermClause),
    Range(RangeClause),
}

#[derive(Debug, PartialEq)]
pub struct TermClause {
    name: String,
    value: String,
}

#[derive(Debug, PartialEq)]
pub struct RangeClause {
    name: String,
    operation: FilterClauseRangeOp,
    value: String,
}

impl Serialize for TermClause {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let TermClause { name, value } = self;
        let mut state = serializer.serialize_struct("TermClause", 1)?;
        let data = serde_json::json!({ name: value });
        state.serialize_field("term", &data)?;
        state.end()
    }
}

impl Serialize for RangeClause {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let RangeClause {
            name,
            operation,
            value,
        } = self;
        let mut state = serializer.serialize_struct("RangeClause", 1)?;
        state.serialize_field("range", &serde_json::json!({ name: { operation: value }}))?;
        state.end()
    }
}

#[derive(Debug, PartialEq)]
pub enum FilterClauseRangeOp {
    Gte,
    Lte,
    Gt,
    Lt,
    Eq,
    Neq,
}

impl From<&FilterClauseRangeOp> for String {
    fn from(op: &FilterClauseRangeOp) -> Self {
        String::from(match op {
            FilterClauseRangeOp::Gte => "gte",
            FilterClauseRangeOp::Lte => "lte",
            FilterClauseRangeOp::Gt => "gt",
            FilterClauseRangeOp::Lt => "lt",
            FilterClauseRangeOp::Eq => "eq",
            FilterClauseRangeOp::Neq => "neq",
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn verify_query_input() {
        println!(
            "{}",
            serde_json::to_string_pretty(&FilterClause::Term(TermClause {
                name: String::from("a"),
                value: String::from("b")
            }))
            .unwrap()
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&FilterClause::Range(RangeClause {
                name: String::from("a"),
                operation: FilterClauseRangeOp::Gte,
                value: String::from("b")
            }))
            .unwrap()
        )
    }
}

pub fn server_address() -> (Ipv4Addr, u16) {
    (
        std::env::var("SERVER_HOST")
            .unwrap_or(String::from("127.0.0.1"))
            .parse()
            .expect("SERVER_HOST should be a valid Ip v4"),
        std::env::var("SERVER_PORT")
            .unwrap_or(String::from("8080"))
            .parse()
            .expect("SERVER_PORT should be a u16"),
    )
}

pub fn db_cfg() -> (String, u16) {
    (
        std::env::var("DB_HOST").unwrap_or(String::from("localhost")),
        std::env::var("DB_HOST")
            .unwrap_or(String::from("9200"))
            .parse()
            .expect("DB_PORT should be a number!"),
    )
}
