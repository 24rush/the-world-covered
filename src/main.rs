use ground_covered::App;
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

/*
fn main() {
    let current_athlete_id = 4399230;
    let app = App::new(current_athlete_id);

    if let None = app.get_athlete_data(current_athlete_id) {
        app.create_athlete(current_athlete_id);
    }

    let _athlete_data = app.get_athlete_data(current_athlete_id).unwrap();

    //app.perform_db_integrity_check();
    app.start_db_pipeline();
}
*/

#[options("/<_..>")]
fn all_options() {
    /* Intentionally left empty */
}

#[get("/routes/<athlete_id>")]
async fn routes(athlete_id: &str) -> (Status, (ContentType, String)) {
    if let Ok(ath_id) = athlete_id.parse::<i64>() {
        if let Some(app) = App::new(ath_id).await {
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

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Cors)
        .mount("/", routes![routes, all_options])
}
