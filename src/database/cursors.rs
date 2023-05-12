use mongodb::bson;


pub struct I64Cursor {
    map_fn: fn(bson::Document) -> i64,
    cursor: mongodb::sync::Cursor<bson::Document>
}

impl I64Cursor {
    pub fn new(cursor: mongodb::sync::Cursor<bson::Document>, map: fn(bson::Document) -> i64) -> Self
    {
        Self { map_fn: map, cursor }
    }
}

impl Iterator for I64Cursor {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.cursor.next() {
            return Some((self.map_fn)(value.unwrap()))
        }

        return None
    }
}