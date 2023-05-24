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
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
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

#[get("/routes/<athlete_id>")]
async fn routes(athlete_id: &str) -> (Status, (ContentType, String)) {
    if let Ok(ath_id) = athlete_id.parse::<i64>() {
        if let Some(app) = App::with_athlete(ath_id).await {
            if let Some(_) = app.get_athlete_data(ath_id).await {
                let routes = app.get_routes(ath_id).await;

                return (
                    Status::Ok,
                    (ContentType::JSON, serde_json::to_string(&routes).unwrap()),
                );
            }
        }
    }

    (Status::NotFound, (ContentType::Text, String::new()))
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
    let efforts = app.query_efforts(parse_query_to_bson(&query)).await;
    (
        Status::Ok,
        (ContentType::JSON, serde_json::to_string(&efforts).unwrap()),
    )
}

/*
#[tokio::main]
async fn main() {
    let app = App::with_athlete(4399230).await;

    //app.unwrap().start_db_integrity_check().await;
    app.unwrap().start_db_creation().await;
}

*/
#[launch]
fn rocket() -> _ {
    rocket::build().attach(Cors).mount(
        "/",
        routes![routes, activities, query_activities, query_efforts, all_options],
    )
}
