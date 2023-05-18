use data_types::{
    gc::route::Route,
    strava::{
        activity::Activity,
        athlete::{AthleteData, AthleteId},
    },
};
use database::{gc_db::GCDB, strava_db::StravaDB};

use processors::Pipeline;
use strava::api::StravaApi;
use util::db_integrity_checks::DBIntegrityChecker;

use crate::{
    database::strava_db::ResourceType,
    util::{
        logging,
        sync_from_strava::{UtilitiesContext},
    },
};

mod data_types;
mod database;
mod strava;
mod util;

mod processors;

pub struct App {
    loggedin_athlete_id: AthleteId,
    strava_api: StravaApi,
    strava_db: StravaDB,
    gc_db: GCDB,
}

impl App {
    const CC: &str = "App";

    pub async fn new(ath_id: AthleteId) -> Option<Self> {
        logging::set_global_level(logging::LogLevel::VERBOSE);

        if let Some(strava_api) = StravaApi::new(ath_id).await {
            return Some(Self {
                loggedin_athlete_id: ath_id,
                strava_api,
                strava_db: StravaDB::new().await,
                gc_db: GCDB::new().await,
            });
        }

        None
    }

    pub async fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        self.strava_db.get_athlete_data(id).await
    }

    pub async fn get_activity(&self, id: i64) -> Option<Activity> {
        self.strava_db.get_activity(id).await
    }

    pub async fn get_routes(&self, ath_id: AthleteId) -> Vec<Route> {
        let mut cursor_routes = self.gc_db.get_routes(ath_id).await;
        let mut routes: Vec<Route> = Vec::new();

        while cursor_routes.advance().await.unwrap() {
            routes.push(cursor_routes.deserialize_current().unwrap())
        }

        routes
    }

    pub async fn create_athlete(&self, id: i64) -> AthleteData {
        let mut default_athlete: AthleteData = Default::default();
        default_athlete._id = id;
        self.strava_db.set_athlete_data(&default_athlete).await;

        default_athlete
    }

    pub async fn store_athlete_activity(&mut self, act_id: i64) {
        logln!("Downloading activity: {}", act_id);

        if let Some(mut new_activity) = self.strava_api.get_activity(act_id).await {
            self.strava_db
                .store_resource(ResourceType::Activity, act_id, &mut new_activity)
                .await;
        }
    }

    pub async fn start_db_pipeline(&self) {
        Pipeline::start(self.loggedin_athlete_id, &self.strava_db, &self.gc_db).await;
    }

    pub async fn perform_db_integrity_check(&mut self) {
        let options = util::db_integrity_checks::Options {
            skip_activity_sync: true,
            skip_activity_telemetry: true,
            skip_segment_caching: false,
            skip_segment_telemetry: true,
        };

        let mut utilities_ctx = UtilitiesContext {
            strava_api: &mut self.strava_api,
            persistance: &self.strava_db,
        };

        DBIntegrityChecker::perform_db_integrity_check(self.loggedin_athlete_id, &options, &mut utilities_ctx).await;
    }
}
