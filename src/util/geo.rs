use std::f32::consts::PI;
use geo_types::Coord;

use crate::data_types::strava::telemetry::{LatLng};

pub struct GeoUtils;

impl GeoUtils {
    pub fn distance(p1: Coord, p2: Coord) -> f64 {
        let lat1 = p1.x;
        let lat2 = p2.x;
        let long1 = p1.y;
        let long2 = p2.y;

        let theta = long2 - long1;

        let mut dist = GeoUtils::deg2rad(lat1).sin() * GeoUtils::deg2rad(lat2).sin()
            + GeoUtils::deg2rad(lat1).cos()
                * GeoUtils::deg2rad(lat2).cos()
                * GeoUtils::deg2rad(theta).cos();

        dist = dist.acos();
        dist = GeoUtils::rad2deg(dist);
        dist = dist * 60.0 * 1.1515;
        dist = dist * 1.609344;

        dist
    }

    pub fn deg2rad(deg: f64) -> f64 {
        deg * PI as f64 / 180.0
    }

    pub fn rad2deg(rad: f64) -> f64 {
        rad * 180.0 / PI as f64
    }

    pub fn get_bounding_box(polyline: &String) -> (Coord, Coord) {
        let line_string = polyline::decode_polyline(&polyline, 5).unwrap();

        let mut min_lat: f64 = 180.0;
        let mut min_long: f64 = 180.0;
        let mut max_lat: f64 = 0.;
        let mut max_long: f64 = 0.;

        line_string.coords().for_each(|coord| {            
            min_lat = coord.x.min(min_lat);
            min_long = coord.y.min(min_long);

            max_lat = coord.x.max(max_lat);
            max_long = coord.y.max(max_long);
        });

        (
            Coord::from((min_lat, min_long)),
            Coord::from((max_lat, max_long)),
        )
    }

    pub fn get_center_of_bbox(left_b: Coord, right_top: Coord) -> Coord {
        Coord::from(((left_b.x + right_top.x) / 2., (left_b.y + right_top.y) / 2.))
    }

    pub fn get_coords_from_poly(polyline: &String) -> Vec<Coord> {
        let line_string = polyline::decode_polyline(&polyline, 5).unwrap();

        line_string.coords().into_iter().cloned().collect()
    }

    // Returns a list of indexes which point to the values present in the polyline received
    pub fn get_index_mapping(polyline: &String, latlngs: &Vec<LatLng>) -> Vec<usize> {
        let poly_coords = GeoUtils::get_coords_from_poly(polyline);

        let mut index_in_poly_coords: usize = 0;
        let mut remapped_indexes: Vec<usize> = vec![0; latlngs.len()];
        let mut coordinates: Vec<Coord> = Vec::new();

        let coord_eq_latlng = |coord: &Coord, latlng: &LatLng| -> bool {
            if (coord.x - latlng[1] as f64).abs() <= 0.00005
                && (coord.y - latlng[0] as f64).abs() <= 0.00005
            {
                return true;
            }

            false
        };

        latlngs
            .iter()
            .enumerate()
            .for_each(|(index_in_telemetry, telem_latlng)| {
                remapped_indexes[index_in_telemetry] = index_in_poly_coords;

                if (index_in_poly_coords as usize) < poly_coords.len()
                    && coord_eq_latlng(&poly_coords[index_in_poly_coords as usize], telem_latlng)
                {
                    coordinates.push(poly_coords[index_in_poly_coords as usize]);
                    index_in_poly_coords += 1;
                }
            });

        remapped_indexes
    }
}