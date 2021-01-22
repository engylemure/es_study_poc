use crate::lib::*;
use actix_web::*;
use std::{collections::HashMap, net::Ipv4Addr};

pub async fn create_user(
    es_client: web::Data<ElasticSearchClient>,
    input: web::Json<UserInput>,
) -> impl Responder {
    let user = User::from_input(input.into_inner());
    match es_client.post("users", &user.id.to_string(), &user).await {
        Ok(result) if result.is_result_type(ESActionResult::Created) => {
            HttpResponse::Ok().json(&user)
        }
        _ => HttpResponse::BadRequest().json(()),
    }
}

pub async fn view_user(
    es_client: web::Data<ElasticSearchClient>,
    web::Path((id)): web::Path<(String)>,
) -> impl Responder {
    match es_client.get::<User>("users", &id).await {
        Ok(info) => {
            dbg!(&info);
            match info.source {
                Some(user) => HttpResponse::Ok().json(&user),
                _ => HttpResponse::NotFound().json(()),
            }
        }
        _ => HttpResponse::BadRequest().json(()),
    }
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
    es_client: web::Data<ElasticSearchClient>,
    input: web::Query<HashMap<String, String>>,
) -> impl Responder {
    match es_client
        .search::<User>(&search_input(input.into_inner()))
        .await
    {
        Ok(result) => {
            let users: Vec<User> = result
                .hits
                .hits
                .into_iter()
                .map(|hit| serde_json::from_value(hit.source))
                .flatten()
                .collect();
            HttpResponse::Ok().json(&users)
        }
        _ => HttpResponse::BadRequest().json(()),
    }
}

#[actix_web::main]
pub async fn main(
) -> std::io::Result<()> {
    let (db_host, db_port) = db_cfg();
    let es_client = ElasticSearchClient::new(db_host, db_port);
    let es_client = web::Data::new(es_client);
    let server_address = server_address();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(es_client.clone())
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            // .wrap(middleware::DefaultHeaders::default())
            .service(web::resource("/users").route(web::post().to(create_user)))
            .service(web::resource("/users/search").route(web::get().to(search_in_user)))
            .service(web::resource("/users/{id}").route(web::get().to(view_user)))
    })
    .bind(&("localhost", server_address.1))?;

    println!(
        "Server started at {}:{}",
        server_address.0, server_address.1
    );
    server.run().await
}