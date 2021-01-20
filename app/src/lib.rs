use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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

    fn search_url(&self, query: &str) -> String {
        format!("{}/_search?q={}", self.address, query)
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

    pub async fn search<T>(&self, query: &str) -> Result<ESSearchResult, ESError>
    where
        T: Serialize + DeserializeOwned,
    {
        match self.client.get(&self.search_url(query)).send().await {
            Ok(resp) => Ok(Self::resp_into_type::<ESSearchResult>(resp).await?),
            _ => Err(ESError::ConnectionError),
        }
    }

    async fn resp_into_type<T>(resp: Response) -> Result<T, ESError>
    where
        T: DeserializeOwned,
    {
        resp.json::<T>().await.map_err(|err| {
            dbg!(err);
            ESError::DeserializationError
        })
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
    pub max_score: f32,
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
