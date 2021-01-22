use crate::lib::*;
use std::{collections::HashMap, net::Ipv4Addr, sync::Arc};
use warp::*;

pub async fn create_user(
    es_client: Arc<ElasticSearchClient>,
    input: UserInput,
) -> Result<impl warp::Reply, Rejection> {
    let user = User::from_input(input);
    Ok(
        match es_client.post("users", &user.id.to_string(), &user).await {
            Ok(result) if result.is_result_type(ESActionResult::Created) => {
                warp::reply::with_status(warp::reply::json(&user), warp::http::StatusCode::CREATED)
            }
            _ => warp::reply::with_status(
                warp::reply::json(&()),
                warp::http::StatusCode::BAD_REQUEST,
            ),
        },
    )
}

pub async fn view_user(
    es_client: Arc<ElasticSearchClient>,
    id: String,
) -> Result<impl warp::Reply, Rejection> {
    Ok(match es_client.get::<User>("users", &id).await {
        Ok(info) => match info.source {
            Some(user) => {
                warp::reply::with_status(warp::reply::json(&user), warp::http::StatusCode::OK)
            }
            _ => {
                warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::NOT_FOUND)
            }
        },
        _ => warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST),
    })
}

fn search_input(input: HashMap<String, String>) -> SearchInput {
    let query_input = match input.get("query") {
        Some(query) => QueryInput::Text(query.to_string()),
        None => {
            let must: Vec<MatchClause> = ["name", "id", "job", "relationship_status", "age"]
                .iter()
                .map(|attr| {
                    input
                        .get(*attr)
                        .map(|val| MatchClause::new(attr.to_string(), val.to_string()))
                })
                .flatten()
                .collect();
            if must.len() > 0 {
                QueryInput::Bool(QueryDSLInput {
                    must: Some(must),
                    ..Default::default()
                })
            } else {
                QueryInput::MatchAll
            }
        }
    };
    SearchInput::new(
        query_input,
        match input.get("size").map(|s| s.parse()) {
            Some(Ok(size)) => Some(size),
            _ => Some(30),
        },
        match input.get("from").map(|s| s.parse()) {
            Some(Ok(size)) => Some(size),
            _ => Some(0),
        },
    )
}

pub async fn search_in_user(
    es_client: Arc<ElasticSearchClient>,
    input: HashMap<String, String>,
) -> Result<impl warp::Reply, Rejection> {
    Ok(match es_client.search::<User>(&search_input(input)).await {
        Ok(result) => {
            let users: Vec<User> = result
                .hits
                .hits
                .into_iter()
                .map(|hit| serde_json::from_value(hit.source))
                .flatten()
                .collect();
            warp::reply::with_status(warp::reply::json(&users), warp::http::StatusCode::OK)
        }
        _ => warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::BAD_REQUEST),
    })
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let (db_host, db_port) = db_cfg();
    let es_client = ElasticSearchClient::new(db_host, db_port);
    let es_client = Arc::new(es_client);
    let es_client = warp::any().map(move || Arc::clone(&es_client));

    let create_user = warp::post()
        .and(es_client.clone())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and_then(create_user);

    let view_user = warp::get()
        .and(es_client.clone())
        .and(warp::path::param())
        .and_then(view_user);

    let search_user = warp::get()
        .and(warp::path("search"))
        .and(es_client.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(search_in_user);

    let routes = warp::path("users")
        .and(create_user.or(search_user).or(view_user))
        .with(warp::log::log("app"));

    let srv_addr = server_address();

    println!("Server started at {}:{}", srv_addr.0, srv_addr.1);
    warp::serve(routes).run(srv_addr).await;
    Ok(())
}