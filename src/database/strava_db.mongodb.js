/* global use, db */
// MongoDB Playground
// To disable this template go to Settings | MongoDB | Use Default Template For Playground.
// Make sure you are connected to enable completions and to be able to run a playground.
// Use Ctrl+Space inside a snippet or a string literal to trigger completions.
// The result of the last command run in a playground is shown on the results panel.
// By default the first 20 documents will be returned with a cursor.
// Use 'console.log()' to print to the debug output.
// For more documentation on playgrounds please refer to
// https://www.mongodb.com/docs/mongodb-vscode/playgrounds/

const database = 'strava_db';
const collection = 'efforts';
// 4399230
// 14.05 1683735145
// 3226579607 9162746293
// Create a new database.

use('strava_db')
db.activities.findOne({ "_id": 9293736762 } );
/*
db.activities.aggregate([
  {
    $match: {
      start_date_local_date: {
        $gte: ISODate("2022-01-01T00:00:00.000Z"),
        $lt: ISODate("2023-01-01T00:00:00.000Z")
      }
    }
  },
  {
    $project: {
      month: {
        $month: "$start_date_local_date"
      },
      distance_in_kms: {
        $divide: [ "$distance", 1000 ]
      }
    }
  },
  {
    $group: {
      _id: "$month",
      totalKs: {
        $sum: "$distance_in_kms"
      }
    }
  },
  {
    $project:{
      _id:0,
      month: "$_id",
      totalKs:1
    }
  }
])
/*
db.activities.aggregate([
  { $match: { start_date_local_date: { $gte: ISODate('2022-01-01'), $lt: ISODate('2023-01-01') }, type: 'Ride' } },
  {
    $project: {
      distanceKM: { $divide: ["$distance", 1000] },
      month: { $month: "$start_date_local_date" }
    }
  },
  {
    $group: {
      _id: "$month",
      totalDistanceKM: { $sum: "$distanceKM" }
    }
  }
])
/*
use('strava_db')
db.activities.aggregate([
  {
    $match: {
      "segment_efforts.segment.id": 33538621
    }
  },
          { $sort: { "segment_efforts.start_date_local": 1 } },

  {
    $addFields: {
      "segment_efforts": {
        "$filter": {
          "input": "$segment_efforts",
          "cond": {
            "$eq": ["$$this.segment.id", 33538621]
          }
        }
      }
    }
  },
  {
    $project: {
      "segment_efforts" : 1
    }
  }
])
//db.athletes.updateOne({"_id": 4399230}, {$set: {"after_ts": 1687071482}})

/*
use('gc_db')

db.activities.aggregate([{ $match: { "athlete.id": 4399230 } }, {
  $sort: { "distance": -1 }
}]);

/*
    db.activities.aggregate([
        {
            $match: {
                type: "Run"
            }
        },
        {
            $group: {
                _id: "$location_city",
                total_distance: {$sum: "$distance"},
                average_speed: {$avg: "$average_speed"},
                total_elevation_gain: {$sum: "$total_elevation_gain"},
                athlete_count: {$sum: "$athlete_count"}
            }
        },
        {
            $sort: {
                total_distance: -1
            }
        }
    ])
 /*
use('gc_db')




db.activities.aggregate([
  {
    $match: {
      $and: [{ type: "Ride" },
      { $or: [ {location_country: { $regex: "Telega" }}, {location_city: { $regex: "Telega" }}] }]
    }
  }
])

db.activities.aggregate([
  {
    $group: {
      _id: "$location_city",
   }
  },
  {$sort: {_id: 1}}
])
/*
//db.routes.find({"master_activity_id": 671296231})

/*

db.activities.aggregate([
  {
    $group: {
      _id: {
        year: {$year: "$start_date_local_date" },
    },

    total_elevation_gain: { $sum: "$total_elevation_gain" }
   }
  }
])

//use('gc_db');
/*db.routes.aggregate([{
$match: {
  'gradients.gradient': { $gt: 7 }
}
}, { $sort: { 'gradients.gradient': -1 } }
])

//db.activities.updateOne({"segment_efforts.id": 66250445483}, {$set: {'segment_efforts.$.start_index_poly':  223 }});
/*
db.telemetry.aggregate([{ $match: { "_id": 6634740828 } }, {
$addFields: { act_count: { $size: { "$ifNull": ["$latlng.data", []] } } }
}]);
*/
//use('gc_db')

//db.activities.find({total_elevation_gain: {$gt: 1000}}).count()
//db.activities.find({location_country: "Romania", athlete_count: {$gt: 1}})
/*



/*14147379 71663221798
db.activities.aggregate([{ $match: { "segment_efforts.segment.id": 8536723 } }, {
  $project: {
    segment_efforts: {
      $filter: {
         input: "$segment_efforts",
         as: "segment_efforts",
         cond: { $eq: [ "$$segment_efforts.segment.id", 8536723 ] }
      }
   }
  }
}])

/*
db.routes.find({
"activities": {
  $in: [375494154,
  ]
}
});

/*
while (res.hasNext()) {
var doc = res.next();
let type = doc["type"];    
 
db.telemetry.updateOne({"_id": doc["_id"] },
  { $set: { "type" : type} },
  {
    upsert: false,
    multi: false
  })
}

/*
db.telemetry.updateMany({},
{ $set: { "athlete": { "id": 4399230 } } },
{
  upsert: false,
  multi: false
})

while (res.hasNext()) {
var doc = res.next();

let found = db.telemetry.findOne({ "_id": doc["_id"] })

if (!found) {
  print("Missing " + doc["_id"]);
}
}
*/

/*
db.athletes.insertOne({"_id" : 4399230, "after_ts" : 1683557390, "before_ts" : 1683557390,
  "tokens" : {
    "access_token": "faa9ec8836d6ba205724bda7f769e49ecfb9a778",
    "refresh_token":"ef9890be0ff863740c50fe0e829409f035cec95b",
    "expires_at" : 1683837165
  }
})
*/
