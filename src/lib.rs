use std::sync::{Arc, RwLock};

use data_types::{
    gc::route::Route,
    strava::{
        activity::Activity,
        athlete::{AthleteData, AthleteId, AthleteTokens},
    },
};
use database::{
    gc_db::GCDB,
    strava_db::{AthletesCollection, StravaDB},
};
use mongodb::bson;
use util::facilities::DependenciesBuilder;

use processors::{
    DataPipeline, DataCreationPipelineOptions, PipelineOperationType, SubOperationType,
};
use strava::api::StravaApi;

use crate::util::logging;

pub mod data_types;
mod database;
mod strava;
mod util;
mod processors;

pub struct TokenExchange {
    athlete_id: AthleteId,
    athletes_collection: AthletesCollection,
    tokens: RwLock<AthleteTokens>,
}

impl TokenExchange {
    async fn new(
        athletes_collection: AthletesCollection,
        athlete_id: AthleteId,
        athlete_tokens: AthleteTokens,
    ) -> TokenExchange {
        return Self {
            athlete_id,
            athletes_collection,
            tokens: RwLock::new(athlete_tokens),
        };
    }

    fn get_tokens(&self) -> AthleteTokens {
        self.tokens.read().unwrap().clone()
    }

    async fn set_tokens(&self, tokens: &AthleteTokens) {
        self.athletes_collection
            .set_athlete_tokens(self.athlete_id, tokens)
            .await;
        *self.tokens.write().unwrap() = tokens.clone();
    }
}

pub struct App {
    loggedin_athlete_id: Option<AthleteId>,
    strava_api: Option<Arc<StravaApi>>,
    strava_db: Arc<StravaDB>,
    gc_db: Arc<GCDB>,
}

impl App {
    const CC: &str = "App";
    const LOCAL_MONGO_URL: &str = "mongodb://localhost:27017";

    pub async fn anonym_athlete() -> App {
        Self {
            loggedin_athlete_id: None,
            strava_api: None,
            strava_db: Arc::new(StravaDB::new(App::LOCAL_MONGO_URL).await),
            gc_db: Arc::new(GCDB::new(App::LOCAL_MONGO_URL).await),
        }
    }

    pub async fn with_athlete(athlete_id: AthleteId) -> Option<App> {
        let mut this = Self {
            loggedin_athlete_id: Some(athlete_id),
            strava_api: None,
            strava_db: Arc::new(StravaDB::new(App::LOCAL_MONGO_URL).await),
            gc_db: Arc::new(GCDB::new(App::LOCAL_MONGO_URL).await),
        };

        if let Some(athlete_data) = this.strava_db.athletes.get_athlete_data(athlete_id).await {
            let token_exchange = TokenExchange::new(
                this.strava_db.get_athletes_collection(),
                athlete_id,
                athlete_data.tokens,
            )
            .await;
        
            this.strava_api = Some(Arc::new(StravaApi::new(token_exchange, athlete_id)));

            return Some(this);
        }

        None
    }

    pub async fn query_activities(
        &self,
        stages: Vec<bson::Document>,
    ) -> Vec<mongodb::bson::Document> {
        self.strava_db
            .activities
            .query_activities_docs(stages)
            .await
    }

    pub async fn query_routes(&self, stages: Vec<bson::Document>) -> Vec<Route> {
        self.gc_db.routes.query(stages).await
    }

    pub async fn query_statistics(&self) -> Vec<mongodb::bson::Document> {
        self.gc_db.statistics.query().await
    }

    pub async fn get_activity(&self, id: i64) -> Option<Activity> {
        self.strava_db.activities.get(id).await
    }

    // Currently unused, to be used when new athletes are uploaded or db rewritten
    pub async fn create_athlete(&self, id: i64) -> AthleteData {
        let mut default_athlete: AthleteData = Default::default();
        default_athlete._id = id;
        self.strava_db
            .athletes
            .set_athlete_data(&default_athlete)
            .await;

        default_athlete
    }

    pub async fn start_data_pipeline(&self) {
        DataPipeline::new(
            DependenciesBuilder::new()
                .with_gc_db(&self.gc_db)
                .with_strava_db(&self.strava_db)
                .with_strava_api(&self.strava_api.clone().unwrap())
                .build(),
            self.loggedin_athlete_id.unwrap(),
        )
        .start(&DataCreationPipelineOptions {
            activity_syncer: PipelineOperationType::Enabled(SubOperationType::None),
            route_matching: PipelineOperationType::Enabled(SubOperationType::Rewrite),
            route_processor: PipelineOperationType::Enabled(SubOperationType::None),
        })
        .await;
    }
}
