const database = 'strava_db';
use(database)

class ActivityMonthlyStats {
    type = "";
    total_km = 0;
    mins_per_week = 0;
}

class ActivityYearlyStats {
    type = "";
    count = 0;

    total_km = 0;
    total_elevation_gain = 0;
    mins_per_week = 0;
    avg_speed = 0;

    calories = 0;

    hardest_ride_id = 0;
    longest_ride_id = 0;
}

class YearStats {
    year = 0;
    sports = []; // ActivityYearlyStats

    vo2max_run = 0;
    best_12min_act_id = 0;

    total_kudos = 0;
    most_kudos_activity = 0;
    rides_with_friends = 0;
    runs_over_20k = 0;
    rides_over_100k = 0;
    rides_over_160k = 0;

    current_month = []; // ActivityMonthlyStats   
}

class WholeStats {
    years_of_sports = []; // YearStats
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
        $gte: new Date(year + "-" + month + "-01T00:00:00Z"),
        $lt: new Date((year + 1) + "-" + month + "-01T00:00:00Z"),
    }
}

function run_query(query) {
    var results = db.activities.aggregate(query).toArray();

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

function acts_with_friends_in_year(year) {
    return [
        {
            $addFields: {
                dateStr: {
                    $dateFromString: {
                        dateString: "$start_date_local"
                    }
                }
            }
        },
        {
            $match:
            {
                $or: [
                    {
                        athlete_count: {
                            $gt: 1
                        },
                        dateStr: date_in_year(year)
                    },
                    {
                        athlete_count: {
                            $gt: 1
                        },
                        start_date_local_date: date_in_year(year)
                    }]
            }
        },
        {
            $project: {
                dateStr: 0
            }
        }, count_stage()];
}

function act_type_in_year(act_type, year, month) {
    return [
        {
            $addFields: {
                dateStr: {
                    $dateFromString: {
                        dateString: "$start_date_local"
                    }
                }
            }
        },
        {
            $match: {
                $or: [
                    {
                        type: act_type,
                        dateStr: date_in_year(year)
                    },
                    {
                        type: act_type,
                        start_date_local_date: date_in_year(year)
                    }
                ]
            }
        },
        {
            $project: {
                dateStr: 0
            }
        }];
}

function act_type_in_year_distance_gt_than(act_type, year, value) {
    return [
        {
            $addFields: {
                dateStr: {
                    $dateFromString: {
                        dateString: "$start_date_local"
                    }
                }
            }
        },
        {
            $match: {
                $or: [
                    {
                        type: act_type,
                        dateStr: date_in_year(year),
                        "distance": { $gte: value }
                    },
                    {
                        type: act_type,
                        start_date_local_date: date_in_year(year),
                        "distance": { $gte: value }
                    }
                ]
            }
        },
        {
            $project: {
                dateStr: 0
            }
        }, count_stage()];
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

function get_vo2max_in_year(year) {
    let max_distance = 0;
    let max_act = 0;
    var results = db.activities.aggregate(act_type_in_year('Run', year)).toArray();

    const TEST_DURATION = 12 * 60;

    results.forEach(element => {
        var telemetry = db.telemetry.find({ "_id": element._id }).toArray()[0];

        if (!telemetry || !telemetry.time) {
            console.log('Failed to process ' + element._id + " " + element.start_date_local);
            return;
        }

        console.log('Processed ' + element._id + " " + element.start_date_local);

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

                    var curr_dist = Math.max(curr_dist, parseInt(telemetry.distance.data[i]) - parseInt(telemetry.distance.data[start_section]));
                    // Avoid abberations
                    if (curr_dist > max_distance && curr_dist <= 3000) {
                        max_distance = curr_dist;
                        max_act = element._id;
                    }
                    start_section += 1;
                }
            }
        }
    });

    return [max_distance > 0 ? (max_distance - 504.9) / 44.73 : 0, max_act];
}

function weeksFromStartOfYear() {
    const now = new Date();
    const startOfYear = new Date(now.getFullYear(), 0, 1); // January 1st of current year

    const msInWeek = 1000 * 60 * 60 * 24 * 7;
    const diffInMs = now - startOfYear;

    const weeks = diffInMs / msInWeek;
    return Math.floor(weeks); // or Math.ceil() to round up
}

var years = range(2014, new Date().getFullYear());
var all_stats = new WholeStats();
var weeks_in_current_year = weeksFromStartOfYear();

for (let year of years) {
    print("Year : " + year);

    var year_stats = new YearStats();
    all_stats.years_of_sports.push(year_stats);

    sports = []; // ActivityYearlyStats

    year_stats.year = year;

    /*
    let vo2max_and_activity = get_vo2max_in_year(year);
    year_stats.vo2max_run = vo2max_and_activity[0];
    year_stats.best_12min_act_id = vo2max_and_activity[1];
    */
    year_stats.total_kudos = run_query(total_field_on_act_type_in_year("Run", "kudos_count", year));
    year_stats.runs_over_20k = run_query(act_type_in_year_distance_gt_than("Run", year, 16000));
    year_stats.rides_over_100k = run_query(act_type_in_year_distance_gt_than("Ride", year, 100000));
    year_stats.rides_over_160k = run_query(act_type_in_year_distance_gt_than("Ride", year, 160000));
    year_stats.rides_with_friends = run_query(acts_with_friends_in_year(year));

    for (let activity_type of ["Ride", "Run"]) {
        var activity_yearly_stats = new ActivityYearlyStats();
        activity_yearly_stats.type = activity_type;
        year_stats.sports.push(activity_yearly_stats);

        activity_yearly_stats.total_elevation_gain = run_query(total_field_on_act_type_in_year(activity_type, "total_elevation_gain", year));
        activity_yearly_stats.total_km = run_query(total_field_on_act_type_in_year(activity_type, "distance", year)) / 1000;
        activity_yearly_stats.mins_per_week = run_query(total_field_on_act_type_in_year(activity_type, "moving_time", year)) / 60 / (year == 2025 ? weeks_in_current_year : 52);
        activity_yearly_stats.calories = run_query(total_field_on_act_type_in_year(activity_type, "calories", year));
        activity_yearly_stats.count = run_query(act_type_in_year_count(activity_type, year));

        var total_speed = run_query(total_field_on_act_type_in_year(activity_type, "average_speed", year));
        activity_yearly_stats.avg_speed = activity_yearly_stats.count ? total_speed / activity_yearly_stats.count : 0;

        print("Activities|" + " " + activity_yearly_stats.count);
        print("AvgS|" + " " + activity_yearly_stats.avg_speed);
        print("TEG  |" + " " + activity_yearly_stats.total_elevation_gain);
        print("Total KMs|" + " " + activity_yearly_stats.total_km);
        print("H Runs |" + " " + activity_yearly_stats.mins_per_week);
        print("Calories |" + " " + activity_yearly_stats.calories);
    }

    if (year == new Date().getFullYear()) {
        var current_month = (new Date().getMonth() + 1).toString().padStart(2, '0');

        for (var activity_type of ["Ride", "Run"]) {
            var month_stats = new ActivityMonthlyStats();

            month_stats.type = activity_type;
            month_stats.total_km = run_query(total_field_on_act_type_in_year(activity_type, "distance", year, current_month)) / 1000;
            month_stats.mins_per_week = run_query(total_field_on_act_type_in_year(activity_type, "moving_time", year, current_month)) / 60 / (year == 2025 ? weeks_in_current_year : 52);

            year_stats.current_month.push(month_stats);

            print("Current month " + activity_type + "|" + month_stats.total_km + " | " + month_stats.mins_per_week);
        }
    }

    print("VO2Max| " + year_stats.vo2max_run);
    print("VO2Max Act| " + year_stats.best_12min_act_id);
    print("W Fri|" + " " + year_stats.rides_with_friends);
    print("Run 20 |" + " " + year_stats.runs_over_20k);
    print("Ride 100 |" + " " + year_stats.rides_over_100k);
    print("Ride 160 |" + " " + year_stats.rides_over_160k);

    print(" ")

}

use('gc_db')
db.statistics.updateOne({ _id: 0 }, { $set: { "stats": all_stats } }, { upsert: true })
