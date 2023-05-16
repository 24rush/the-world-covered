use ground_covered::App;

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


