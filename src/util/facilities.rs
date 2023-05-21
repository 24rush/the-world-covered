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

pub struct Facilities<'a> {
    strava_api: Option<&'a StravaApi>,
    strava_db: Option<&'a StravaDB>,
    gc_db: Option<&'a GCDB>,
}

impl Facilities<'_> {
    pub fn strava_db(&self) -> &StravaDB {
        self.strava_db.unwrap()
    }

    pub fn gc_db(&self) -> &GCDB {
        self.gc_db.unwrap()
    }

    pub fn strava_api(&self) -> &StravaApi {
        self.strava_api.unwrap()
    }

    pub fn check(&self, required: Vec<Required>) {
        for depend in &required {
            match (*depend) as usize {
                0 => {
                    self.strava_api.expect("Expecting Strava API");
                }
                1 => {
                    self.strava_db.expect("Expecting Strava database");
                }
                2 => {
                    self.gc_db.expect("Expecting GC database");
                }
                _ => panic!("Unknown requirement"),
            }
        }
    }
}

pub struct DependenciesBuilder<'a> {
    dependencies: Facilities<'a>,
}

impl<'a> DependenciesBuilder<'a> {
    const CC: &str = "DependenciesBuilder";

    pub fn new() -> Self {
        Self {            
            dependencies: Facilities {
                strava_api: None,
                strava_db: None,
                gc_db: None,
            },
        }
    }

    pub fn with_strava_db(
        &'a mut self,
        strava_db: &'a StravaDB,
    ) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.strava_db = Some(strava_db);
        self
    }

    pub fn with_gc_db(&'a mut self, gc_db: &'a GCDB) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.gc_db = Some(gc_db);
        self
    }

    pub fn with_strava_api(
        &'a mut self,
        strava_api: &'a StravaApi,
    ) -> &'a mut DependenciesBuilder<'a> {
        self.dependencies.strava_api = Some(strava_api);
        self
    }

    pub fn build(&mut self) -> &mut Facilities<'a> {
        &mut self.dependencies
    }
}
