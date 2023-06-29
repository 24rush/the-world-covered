use std::{sync::Arc};
use crate::{
    database::{gc_db::GCDB, strava_db::StravaDB},
    strava::api::StravaApi,
};

#[derive(PartialEq, Copy, Clone)]
#[repr(usize)]
pub enum Required {
    StravaApi = 0,
    StravaDB,
    GcDB,
}

pub struct Facilities {
    strava_api: Option<Arc<StravaApi>>,
    strava_db: Option<Arc<StravaDB>>,
    gc_db: Option<Arc<GCDB>>,
}

impl Facilities {
    pub fn strava_db(&self) -> Arc<StravaDB> {
        Arc::clone(&self.strava_db.as_ref().unwrap())
    }

    pub fn gc_db(&self) -> Arc<GCDB> {
        Arc::clone(self.gc_db.as_ref().unwrap())
    }

    pub fn strava_api(&self) -> Arc<StravaApi> {
        Arc::clone(self.strava_api.as_ref().unwrap())
    }

    pub fn clone(&self) -> Facilities {
        Self {
            strava_api: Some(self.strava_api()),
            strava_db: Some(self.strava_db()),
            gc_db: Some(self.gc_db()),
        }
    }

    pub fn check(&self, required: Vec<Required>) {
        for depend in &required {
            match (*depend) as usize {
                0 => {
                    self.strava_api.as_ref().expect("Expecting Strava API");
                }
                1 => {
                    self.strava_db.as_ref().expect("Expecting Strava database");
                }
                2 => {
                    self.gc_db.as_ref().expect("Expecting GC database");
                }
                _ => panic!("Unknown requirement"),
            }
        }
    }
}

pub struct DependenciesBuilder {
    dependencies: Facilities,
}

impl DependenciesBuilder {
    const CC: &str = "DependenciesBuilder";

    pub fn new() -> Self {
        Self {
            dependencies: Facilities {
                strava_api: None,
                strava_db: None,
                gc_db: None,
            }
            .into(),
        }
    }

    pub fn with_strava_db(&mut self, strava_db: &Arc<StravaDB>) -> &mut DependenciesBuilder {
        self.dependencies.strava_db = Some(Arc::clone(strava_db));
        self
    }

    pub fn with_gc_db(&mut self, gc_db: &Arc<GCDB>) -> &mut DependenciesBuilder {
        self.dependencies.gc_db = Some(Arc::clone(gc_db));
        self
    }

    pub fn with_strava_api(&mut self, strava_api: &Arc<StravaApi>) -> &mut DependenciesBuilder {
        self.dependencies.strava_api = Some(Arc::clone(strava_api));
        self
    }

    pub fn build(&mut self) -> Facilities {
        self.dependencies.clone()
    }
}
