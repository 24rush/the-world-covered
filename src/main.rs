use ground_covered::App;
use rocket::http::{Status, ContentType};

#[macro_use] extern crate rocket;

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

#[get("/routes/<athlete_id>")]
async fn routes(athlete_id: &str) -> (Status, (ContentType, String)) {
    if let Ok(ath_id) = athlete_id.parse::<i64>() {
        let app = App::new(ath_id).await;

        if let Some(_) = app.get_athlete_data(ath_id).await {
            let routes = app.get_routes(ath_id).await;

            return (Status::Ok, (ContentType::JSON, serde_json::to_string(&routes).unwrap()))
        }
    }

    (Status::NotFound, (ContentType::Text, String::new()))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![routes])
}