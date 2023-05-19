use crate::{database::{strava_db::StravaDB, gc_db::GCDB}, strava::api::StravaApi, AthleteId};

#[derive(PartialEq)]
pub enum Required {
    None,
    AthleteId,
    StravaApi,
    StravaDB,
    GCDB,
}

pub struct Dependencies<'a> {
    pub athlete_id: Option<AthleteId>,
    pub strava_api: Option<&'a mut StravaApi>,
    pub strava_db: Option<&'a StravaDB>,
    pub gc_db: Option<&'a GCDB>,
}

impl Dependencies<'_> {
    pub fn strava_db(&self) -> &StravaDB {    
        self.strava_db.unwrap()
    }

    pub fn gc_db(&self) -> &GCDB {
        self.gc_db.unwrap()
    }

    pub fn strava_api(&mut self) -> &mut StravaApi {
        self.strava_api.as_mut().unwrap()
    }

    pub fn athlete_id(&self) -> AthleteId {
        self.athlete_id.unwrap()
    }
}

pub struct DependenciesBuilder<'a> {
    dependencies: Dependencies<'a>,
    required: Vec<Required>,
}

impl<'a> DependenciesBuilder<'a> {
    pub fn new(required: Vec<Required>) -> Self {
        Self {
            required,
            dependencies: Dependencies {
                athlete_id: None,
                strava_api: None,
                strava_db: None,
                gc_db: None
            },
        }
    }

    pub fn with_athlete_id(&'a mut self, ath_id: AthleteId) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.athlete_id = Some(ath_id);
        self
    }

    pub fn with_strava_db(
        &'a mut self,
        strava_db: &'a StravaDB,
    ) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.strava_db = Some(strava_db);
        self
    }

    pub fn with_gc_db(
        &'a mut self,
        gc_db: &'a GCDB,
    ) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.gc_db = Some(gc_db);
        self
    }

    pub fn with_strava_api(
        &'a mut self,
        strava_api: &'a mut StravaApi,
    ) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.strava_api = Some(strava_api);
        self
    }

    pub fn build(&mut self) -> &mut Dependencies<'a> {
        self.ensure_dependencies_injected();

        &mut self.dependencies
    }

    fn ensure_dependencies_injected(&mut self) {
        if self.required.contains(&Required::AthleteId) {
            self.dependencies
                .athlete_id
                .expect("Pipeline expects athlete ID");
        }
        if self.required.contains(&Required::StravaDB) {
            self.dependencies
                .strava_db
                .expect("Pipeline expects Strava database");
        }
        if self.required.contains(&Required::GCDB) {
            self.dependencies
                .gc_db
                .expect("Pipeline expects GC database");
        }
        if self.required.contains(&Required::StravaApi) {
            self.dependencies
                .strava_api
                .as_ref()
                .expect("Pipeline expects Strava API");
        }
    }
}
