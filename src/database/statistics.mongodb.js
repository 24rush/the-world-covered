const database = 'strava_db';
use(database)

class YearStats {
    year = 0;
    rides_with_friends = 0;
    runs = 0;
    rides = 0;
    total_elevation_gain = 0;

    avg_speed_rides = 0;
    avg_speed_runs = 0;
    
    total_km_rides = 0;
    total_km_runs = 0;

    hours_per_week_rides = 0;
    hours_per_week_runs = 0;

    total_kudos = 0;
    most_kudos_activity = 0;
    ////

    //TODO distribution per days of week
    countries_visited = 0;
    states_visited = 0;


    hardest_ride_id = 0;
    longest_ride_id = 0;
}

function range(start, end) {
    var ans = [];
    for (let i = start; i <= end; i++) {
        ans.push(i);
    }
    return ans;
}

function date_in_year(year) {
    return {
        $gte: ISODate(year + "-01-01"),
        $lt: ISODate((year + 1) + "-01-01"),
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

function act_type_in_year(act_type, year) {
    return [{
        $match: {
            type: act_type,
            start_date_local_date: date_in_year(year)
        }
    }];
}

function act_type_in_year_count(act_type, year) {
    var query = act_type_in_year(act_type, year);

    query.push(count_stage());

    return query;
}

function total_field_on_act_type_in_year(act_type, field, year) {
    var query = act_type_in_year(act_type, year);

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

var years = range(2014, 2023);
var yearly_stats = [];

for (let year of years) {
    var year_stats = new YearStats();
    yearly_stats.push(year_stats);
    year_stats.year = year;

    year_stats.rides_with_friends = run_query(acts_with_friends_in_year(year));
    year_stats.runs = run_query(act_type_in_year_count("Run", year));
    year_stats.rides = run_query(act_type_in_year_count("Ride", year));

    year_stats.total_elevation_gain = run_query(total_field_on_act_type_in_year("Ride", "total_elevation_gain", year));
    year_stats.total_km_rides = run_query(total_field_on_act_type_in_year("Ride", "distance", year)) / 1000;
    year_stats.total_km_runs = run_query(total_field_on_act_type_in_year("Run", "distance", year)) / 1000;

    year_stats.hours_per_week_rides = run_query(total_field_on_act_type_in_year("Ride", "moving_time", year)) / 60 / (year == 2023 ? 20 : 52);
    year_stats.hours_per_week_runs = run_query(total_field_on_act_type_in_year("Run", "moving_time", year)) / 60 / (year == 2023 ? 20 : 52);

    year_stats.total_kudos = run_query(total_field_on_act_type_in_year("Run", "kudos_count", year));
    year_stats.most_kudos_activity = run_query(sort_on_field_return_id("kudos_count", year));

    var total_speed = run_query(total_field_on_act_type_in_year("Ride", "average_speed", year));
    year_stats.avg_speed_rides = total_speed / year_stats.rides;
    total_speed = run_query(total_field_on_act_type_in_year("Run", "average_speed", year));
    year_stats.avg_speed_runs = year_stats.runs ? total_speed / year_stats.runs : 0;

    print(year)
    print("W Fri|" + " " + year_stats.rides_with_friends);
    print("Rides|" + " " + year_stats.rides);
    print("AvgS|" + " " + year_stats.avg_speed_rides);
    print("Runs |" + " " + year_stats.runs);
    print("TEG  |" + " " + year_stats.total_elevation_gain);

    print("Rides|" + " " + year_stats.total_km_rides);
    print("Runs |" + " " + year_stats.total_km_runs);

    print("H Runs |" + " " + year_stats.hours_per_week_runs);
    print("H Ride |" + " " + year_stats.hours_per_week_rides);

    print("Kudos |" + " " + year_stats.total_kudos);
    print("Act |" + " " + year_stats.most_kudos_activity);

    print(" ")

}

use('gc_db')
db.statistics.updateOne({_id: 0}, {$set: {"stats":yearly_stats}}, {upsert: true})
