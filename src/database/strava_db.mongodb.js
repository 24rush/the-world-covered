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
const collection = 'segments';

// Create a new database.
use(database);

// Create a new collection.
//db.createCollection(collection);

//let old = db.activities.findOne( {"_id": 2439073509})

/*
while (res.hasNext()) {
  var doc = res.next();

  let found = db.telemetry.findOne({ "_id": doc["_id"] })

  if (!found) {
    print("Missing " + doc["_id"]);
  }
}
*/


db.athletes.insertOne({"_id" : 4399230, "after_ts" : 1683557390, "before_ts" : 1683557390,
  "tokens" : {
    "access_token": "faa9ec8836d6ba205724bda7f769e49ecfb9a778",
    "refresh_token":"ef9890be0ff863740c50fe0e829409f035cec95b",
    "expires_at" : 1683837165
  }
})

