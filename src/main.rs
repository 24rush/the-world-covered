use ground_covered::App;

fn main() {
    let current_athlete_id = 4399230;
    let app = App::new(current_athlete_id);

    if let None = app.get_athlete(current_athlete_id) {
        app.create_athlete(current_athlete_id);
    }

    let athlete_data = app.get_athlete(current_athlete_id).unwrap();
    println!("AthleteData: {:#?}", athlete_data);
    
    //app.sync_athlete_activities(&current_athlete_id);
    //app.check_database_integrity();    
}

