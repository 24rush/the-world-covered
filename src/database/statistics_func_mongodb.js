const serviceName = "mongodb-atlas";
const strava_db = "strava_db";
const gc_db = "gc_db";

const activities_collName = "activities";
const telemetry_collName = "telemetry";
const statistics_collName = "statistics";

class ActivityMonthlyStats {
  constructor() {
    this.type = "";
    this.total_km = 0;
    this.mins_per_week = 0;
  }
}

class ActivityYearlyStats {
  constructor() {
    this.type = "";
    this.count = 0;

    this.total_km = 0;
    this.total_elevation_gain = 0;
    this.mins_per_week = 0;
    this.avg_speed = 0;

    this.calories = 0;

    this.hardest_ride_id = 0;
    this.longest_ride_id = 0;
  }
}

class YearStats {
  constructor() {
    this.year = 0;
    this.sports = []; // ActivityYearlyStats

    this.vo2max_run = 0;
    this.best_12min_act_id = 0;

    this.total_kudos = 0;
    this.most_kudos_activity = 0;
    this.rides_with_friends = 0;
    this.runs_over_20k = 0;
    this.rides_over_100k = 0;
    this.rides_over_160k = 0;

    this.current_month = []; // ActivityMonthlyStats   
  }
}

class WholeStats {
  constructor() {
    this.years_of_sports = []; // YearStats
  }
}

function range(start, end) {
    var ans = [];
    for (let i = start; i <= end; i++) {
        ans.push(i);
    }
    return ans;
}

function date_in_year(year, month = "01") {
    return {
      "$gte": new Date(year + "-" + month + "-01T00:00:00.000Z"),
      "$lt": new Date((year + 1) + "-" + month + "-01T00:00:00.000Z"),
    }
}

async function run_query(query) {
    var activities_coll = context.services.get(serviceName).db(strava_db).collection(activities_collName);
    var results = await activities_coll.aggregate(query).toArray().then(result => {return result;});

    return (results && results.length) ? results[0].results : 0;
}


function count_stage() {
    return {
        $count: 'results'
    }
}

function sort_stage() {
    return {
        $sort:
        {
            kudos_count: -1,
        }
    }
}

function limit_stage(count) {
    return {
        $limit: count
    }
}

function project_id_as_result_stage() {
    return {
        $project: {
            _id: 0,
            "results": '$_id'
        }
    }
}

function acts_in_year(year) {
    return [{
        $match: {
            start_date_local_date: date_in_year(year)
        }
    }];
}

function acts_with_friends_in_year(year) {
    return [{
        $match: {
            athlete_count: {
                $gt: 1
            },
            start_date_local_date: date_in_year(year)
        }
    }, count_stage()];
}

function act_type_in_year(act_type, year, month) {
    return [{
        "$match": {
            type: act_type,
            start_date_local_date: date_in_year(year, month)
        }
    }];
}

function act_type_in_year_distance_gt_than(act_type, year, value) {
    var query = [{
        $match: {
            type: act_type,
            start_date_local_date: date_in_year(year),
            "distance": { $gte: value }
        }
    }];

    query.push(count_stage());

    return query;
}

function act_type_in_year_count(act_type, year) {
    var query = act_type_in_year(act_type, year);

    query.push(count_stage());

    return query;
}

function total_field_on_act_type_in_year(act_type, field, year, month = "01") {
    var query = act_type_in_year(act_type, year, month);

    query.push({
        $group:
        {
            _id: null,
            results: { $sum: "$" + field },
        }
    });

    return query;
}

function max_field_on_act_type_in_year(act_type, field, year) {
    var query = act_type_in_year(act_type, year);

    query.push({
        $sort:
        {
            field: 1,
        }
    });

    return query;
}

function sort_on_field_return_id(field, year) {
    var query = acts_in_year(year);
    query = query.concat(sort_stage(), limit_stage(1), project_id_as_result_stage());

    return query;
}

async function get_vo2max_in_year(year) {
    let max_distance = 0;
    let max_act = 0;

    var activities_coll = context.services.get(serviceName).db(strava_db).collection(activities_collName);
    var telemetry_coll = context.services.get(serviceName).db(strava_db).collection(telemetry_collName);
    
    var results = await activities_coll.aggregate(act_type_in_year('Run', year)).toArray().then(result => { return result;});
    
    const TEST_DURATION = 12 * 60;

    for (var element of results) { 
        var telemetry = await telemetry_coll.findOne({ "_id": element._id }).then(result => {return result;});

        let start_section = -1;
        var curr_dist = 0;

        for (var i = 0; i < telemetry.time.data.length; i++) {
            if (start_section == -1 && parseInt(telemetry.time.data[i]) >= TEST_DURATION) {
                start_section = 0;
            }

            if (start_section != -1) {
                var time_gap = parseInt(telemetry.time.data[i]) - parseInt(telemetry.time.data[start_section]);

                if (time_gap >= TEST_DURATION) {
                    // Compute jump
                    let jump_required = time_gap - TEST_DURATION;
                    let time_at_start_section = parseInt(telemetry.time.data[start_section]);
                    while (parseInt(telemetry.time.data[++start_section] - time_at_start_section) <= jump_required);

                    curr_dist = Math.max(curr_dist, parseInt(telemetry.distance.data[i]) - parseInt(telemetry.distance.data[start_section]));
                    // Avoid abberations
                    if (curr_dist > max_distance && curr_dist <= 3000) {
                        max_distance = curr_dist;
                        max_act = element._id;
                    }
                    start_section += 1;
                }
            }
        }
    }

    return [max_distance > 0 ? (max_distance - 504.9) / 44.73 : 0, max_act];
}

exports = async function() {
  var years = range(2014, 2023);
  var all_stats = new WholeStats();
  
  for (let year of years) {
    var year_stats = new YearStats();
    all_stats.years_of_sports.push(year_stats);
    year_stats.year = year;

    let vo2max_and_activity = await get_vo2max_in_year(year);
    year_stats.vo2max_run = vo2max_and_activity[0];
    year_stats.best_12min_act_id = vo2max_and_activity[1];

    year_stats.total_kudos = await run_query(total_field_on_act_type_in_year("Run", "kudos_count", year));
    year_stats.most_kudos_activity = await run_query(sort_on_field_return_id("kudos_count", year));
    year_stats.runs_over_20k = await run_query(act_type_in_year_distance_gt_than("Run", year, 16000));
    year_stats.rides_over_100k = await run_query(act_type_in_year_distance_gt_than("Ride", year, 100000));
    year_stats.rides_over_160k = await run_query(act_type_in_year_distance_gt_than("Ride", year, 160000));
    year_stats.rides_with_friends = await run_query(acts_with_friends_in_year(year));

    var weeks_in_current_year = 32;
    for (let activity_type of ["Ride", "Run"]) {
        var activity_yearly_stats = new ActivityYearlyStats();
        activity_yearly_stats.type = activity_type;
        year_stats.sports.push(activity_yearly_stats);

        activity_yearly_stats.total_elevation_gain = await run_query(total_field_on_act_type_in_year(activity_type, "total_elevation_gain", year));
        activity_yearly_stats.total_km = await run_query(total_field_on_act_type_in_year(activity_type, "distance", year)) / 1000;
        activity_yearly_stats.mins_per_week = await run_query(total_field_on_act_type_in_year(activity_type, "moving_time", year)) / 60 / (year == 2023 ? weeks_in_current_year : 52);
        activity_yearly_stats.calories = await run_query(total_field_on_act_type_in_year(activity_type, "calories", year));
        activity_yearly_stats.count = await run_query(act_type_in_year_count(activity_type, year));

        var total_speed = await run_query(total_field_on_act_type_in_year(activity_type, "average_speed", year));
        activity_yearly_stats.avg_speed = activity_yearly_stats.count ? total_speed / activity_yearly_stats.count : 0;
    }

    if (year == new Date().getFullYear()) {
        var current_month = (new Date().getMonth() + 1).toString().padStart(2, '0');

        for (var activity_type of ["Ride", "Run"]) {
            var month_stats = new ActivityMonthlyStats();

            month_stats.type = activity_type;
            month_stats.total_km = await run_query(total_field_on_act_type_in_year(activity_type, "distance", year, current_month)) / 1000;
            month_stats.mins_per_week = await run_query(total_field_on_act_type_in_year(activity_type, "moving_time", year, current_month)) / 60 / (year == 2023 ? 32 : 52);

            year_stats.current_month.push(month_stats);
        }
    }
  }
  
  var statistics_coll = context.services.get(serviceName).db(gc_db).collection(statistics_collName);
  statistics_coll.updateOne({ _id: 0 }, { $set: { "stats": all_stats } }, { upsert: true })
};