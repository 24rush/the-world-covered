use data_types::{
    gc::{route::Route, effort::Effort},
    strava::{
        activity::Activity,
        athlete::{AthleteData, AthleteId},
    },
};
use database::{gc_db::GCDB, strava_db::StravaDB};
use maintenance::db_integrity_checks::Options;
use mongodb::bson;
use util::facilities::{DependenciesBuilder};

use crate::maintenance::db_integrity_checks::DBIntegrityChecker;
use processors::{DataCreationPipeline, DataCreationPipelineOptions};
use strava::api::StravaApi;

use crate::{database::strava_db::ResourceType, util::logging};

pub mod data_types;
mod database;
mod strava;
mod util;

mod maintenance;
mod processors;

pub struct App {
    loggedin_athlete_id: Option<AthleteId>,
    strava_api: Option<StravaApi>,
    strava_db: StravaDB,
    gc_db: GCDB,
}

impl App {
    const CC: &str = "App";

    pub async fn anonym_athlete() -> Self {
        Self {
            loggedin_athlete_id: None,
            strava_api: None,
            strava_db: StravaDB::new().await,
            gc_db: GCDB::new().await,
        }
    }

    pub async fn with_athlete(ath_id: AthleteId) -> Option<Self> {
        logging::set_global_level(logging::LogLevel::VERBOSE);

        if let Some(strava_api) = StravaApi::new(ath_id).await {
            return Some(Self {
                loggedin_athlete_id: Some(ath_id),
                strava_api: Some(strava_api),
                strava_db: StravaDB::new().await,
                gc_db: GCDB::new().await,
            });
        }

        None
    }

    pub async fn query_activities(&self, stages: Vec<bson::Document>) -> Vec<Activity> {
        self.strava_db.query_activities(stages).await
    }

    pub async fn query_efforts(&self, stages: Vec<bson::Document>) -> Vec<Effort> {
        self.gc_db.query_efforts(stages).await
    }

    pub async fn query_routes(&self, stages: Vec<bson::Document>) -> Vec<Route> {
        self.gc_db.query_routes(stages).await
    }

    pub async fn query_statistics(&self) -> mongodb::bson::Document {
        self.gc_db.query_statistics().await
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

        if let Some(mut new_activity) = self.strava_api.as_ref().unwrap().get_activity(act_id).await {
            self.strava_db
                .store_resource(ResourceType::Activity, act_id, &mut new_activity)
                .await;
        }
    }

    pub async fn start_db_creation(&self) {
        DataCreationPipeline::new(
            DependenciesBuilder::new()
                .with_gc_db(&self.gc_db)
                .with_strava_db(&self.strava_db)
                .build(),
        )
        .start(
            self.loggedin_athlete_id.unwrap(),
            &DataCreationPipelineOptions {
                commonalities: false,
                route_processor: false,
                statistics: true
            },
        )
        .await;
    }

    pub async fn start_db_integrity_check(&mut self) {
        DBIntegrityChecker::new(
            DependenciesBuilder::new()
                .with_strava_api(&mut self.strava_api.as_ref().unwrap())
                .with_strava_db(&self.strava_db)
                .build(),
            &Options {
                skip_activity_sync: false,
                skip_activity_telemetry: false,
                skip_segment_caching: false,
                skip_segment_telemetry: true,
            },
        )
        .start(self.loggedin_athlete_id.unwrap())
        .await;
    }
}
