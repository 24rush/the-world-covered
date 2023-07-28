use ground_covered::data_types::common::DocumentId;
use ground_covered::App;
use mongodb::bson::{self};
use rocket::http::{ContentType, Status};

#[macro_use]
extern crate rocket;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Cross-Origin-Resource-Sharing Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new(
            "Access-Control-Allow-Origin",
            "http://localhost:5173",
        ));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, PATCH, PUT, DELETE, HEAD, OPTIONS, GET",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[options("/<_..>")]
fn all_options() {
    /* Intentionally left empty */
}

#[get("/activities/<act_id>")]
async fn activities(act_id: &str) -> (Status, (ContentType, String)) {
    if let Ok(act_id) = act_id.parse::<i64>() {
        let app = App::anonym_athlete().await;
        if let Some(activity) = app.get_activity(act_id).await {
            return (
                Status::Ok,
                (ContentType::JSON, serde_json::to_string(&activity).unwrap()),
            );
        }
    }

    (Status::NotFound, (ContentType::Text, String::new()))
}

fn parse_query_to_bson(query: &String) -> Vec<bson::Document> {
    let json: Vec<serde_json::Map<String, serde_json::Value>> =
        serde_json::from_str(query.as_str()).unwrap();

    let mut bsons: Vec<bson::Document> = Vec::new();
    for key in &json {
        let bson = bson::to_document(key).unwrap();

        bsons.push(bson);
    }

    bsons
}

#[post("/query_activities", data = "<query>")]
async fn query_activities(query: String) -> (Status, (ContentType, String)) {
    let app = App::anonym_athlete().await;
    let activities = app.query_activities(parse_query_to_bson(&query)).await;
    (
        Status::Ok,
        (
            ContentType::JSON,
            serde_json::to_string(&activities).unwrap(),
        ),
    )
}

#[post("/query_efforts", data = "<query>")]
async fn query_efforts(query: String) -> (Status, (ContentType, String)) {
    let app = App::anonym_athlete().await;
    let efforts = app.query_activities(parse_query_to_bson(&query)).await;
    (
        Status::Ok,
        (ContentType::JSON, serde_json::to_string(&efforts).unwrap()),
    )
}

#[post("/query_routes", data = "<query>")]
async fn query_routes(query: String) -> (Status, (ContentType, String)) {
    let app = App::anonym_athlete().await;
    let efforts = app.query_routes(parse_query_to_bson(&query)).await;
    (
        Status::Ok,
        (ContentType::JSON, serde_json::to_string(&efforts).unwrap()),
    )
}

#[post("/query_statistics")]
async fn query_statistics() -> (Status, (ContentType, String)) {
    let app = App::anonym_athlete().await;
    let efforts = app.query_statistics().await;
    (
        Status::Ok,
        (ContentType::JSON, serde_json::to_string(&efforts).unwrap()),
    )
}

#[post("/on_activity_updated", data = "<query>")]
async fn on_activity_updated(query: String) -> (Status, (ContentType, String)) {
    let json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(query.as_str()).unwrap();

    let mut is_activity_creation = false;
    let mut is_activity_deletion = false;

    let mut athlete_id: Option<DocumentId> = None;
    let mut act_id: Option<DocumentId> = None;

    for key in &json {
        match key.0.trim() {
            "create" => {
                is_activity_creation = true;
                act_id = key.1.to_string().parse::<i64>().ok();
            }
            "delete" => {
                is_activity_deletion = true;
                act_id = key.1.to_string().parse::<i64>().ok();
            }
            "athlete_id" => {
                athlete_id = key.1.to_string().parse::<i64>().ok();
            }
            _ => {}
        }
    }

    if let Some(athlete_id) = athlete_id {
        if let Some(act_id) = act_id {                        
            if let Some(app) = App::with_athlete(athlete_id).await {
                if is_activity_creation {
                    app.on_new_activity(act_id).await;
                }

                if is_activity_deletion {
                    app.on_delete_activity(act_id).await;
                }
            }
        }
    }

    (
        Status::Ok,
        (ContentType::Text, "OK".to_string()),
    )
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .configure(
            rocket::Config::figment()
                .merge(("port", 8080))
                .merge(("address", "0.0.0.0")),
        )
        .attach(Cors)
        .mount(
            "/",
            routes![
                activities,
                query_routes,
                query_activities,
                query_efforts,
                query_statistics,
                on_activity_updated,
                all_options
            ],
        )
}
