use chrono::{NaiveDateTime, Datelike};

use crate::{
    data_types::{gc::stats::YearlyStats, strava::athlete::AthleteId},
    util::facilities::{Facilities, Required}, logln,
};

pub struct Statistics<'a> {
    dependencies: &'a mut Facilities<'a>,
}

impl<'a> Statistics<'a> {
    const CC: &str = "Statistics";

    pub fn new(dependencies: &'a mut Facilities<'a>) -> Self {
        dependencies.check(vec![Required::GcDB, Required::StravaDB]);

        Self { dependencies }
    }

    pub async fn collect_yearly_stats(&self, ath_id: AthleteId) -> Vec<YearlyStats> {
        let yearly_stats: Vec<YearlyStats> = Vec::new();

        // Get all matched activities and fill in all the efforts
        let mut activities = self
            .dependencies
            .strava_db()
            .get_athlete_activities(ath_id)
            .await;

        let mut counter = 0;
        while activities.advance().await.unwrap() {
            let activity = activities.deserialize_current().unwrap();
            logln!("{}", activity.start_date_local);
            
            let naive_datetime = NaiveDateTime::parse_from_str(&activity.start_date_local, "%Y-%m-%dT%H:%M:%SZ").unwrap();

            logln!("{:?}", naive_datetime.year());

            if counter > 5 {
                break;
            }

            counter +=1;
        }

        yearly_stats
    }
}
