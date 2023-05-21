use crate::{
    data_types::{common::Identifiable, gc::route::Gradient, strava::telemetry::Telemetry},
    logln,
};

pub struct GradientFinder {}

impl GradientFinder {
    const CC: &str = "GradientFinder";

    const CLIMB_GRADIENT_THRESHOLD: f32 = 6.0;
    const DESCENT_GRADIENT_THRESHOLD: f32 = -3.0;
    const DIST_GRADIENT_FLUCT_ALLOWANCE: f32 = 600.0; // meters
    const GRADIENT_MIN_LENGTH_ASC: f32 = 900.0; // meters
    const GRADIENT_MIN_LENGTH_DESC: f32 = 2000.0; // meters

    pub fn find_gradients(telemetry: &Telemetry) -> Vec<Gradient> {
        let mut gradients: Vec<Gradient> = Vec::new();

        logln!("Gradient finder on {:?}", telemetry.as_i64());

        let distance_at = |index: usize| -> f32 { telemetry.distance.data[index] };
        let altitude_at = |index: usize| -> f32 { telemetry.altitude.data[index] };
        let gradient_at = |index: usize| -> f32 { telemetry.grade_smooth.data[index] };

        let distance_between = |p1: usize, p2: usize| -> f32 { distance_at(p2) - distance_at(p1) };
        let gradient_between = |p1: usize, p2: usize| -> f32 {
            100.0 * ((altitude_at(p2) - altitude_at(p1)) / (distance_at(p2) - distance_at(p1)))
        };

        #[derive(PartialEq)]
        enum GradientType {
            None,
            Asc,
            Desc,
        }

        let mut curr_gradient_start_index: usize = 0;
        let mut curr_gradient_end_index: usize = 0;
        let mut in_gradient = false; // No gradient segment started
        let mut curr_gradient_type = GradientType::None;

        let get_gradient_type = |gradient: f32| -> GradientType {
            if gradient >= GradientFinder::CLIMB_GRADIENT_THRESHOLD {
                return GradientType::Asc;
            }

            if gradient <= GradientFinder::DESCENT_GRADIENT_THRESHOLD {
                return GradientType::Desc;
            }

            return GradientType::None;
        };

        let gradient_flipped = |gradient: f32, type_gradient_curr_in: &GradientType| -> bool {
            if *type_gradient_curr_in == GradientType::Asc {
                return gradient < GradientFinder::CLIMB_GRADIENT_THRESHOLD;
            }

            if *type_gradient_curr_in == GradientType::Desc {
                return gradient > GradientFinder::DESCENT_GRADIENT_THRESHOLD;
            }

            false
        };

        let min_gradient_length_for_type = |gradient_type: &GradientType| {
            if *gradient_type == GradientType::Asc {
                GradientFinder::GRADIENT_MIN_LENGTH_ASC
            } else {
                GradientFinder::GRADIENT_MIN_LENGTH_DESC
            }
        };

        let try_extend_gradient_with_point =
            |gradient_type: &GradientType, p1_index: usize, p2_index: usize| -> usize {
                let elev_diff = altitude_at(p2_index) - altitude_at(p1_index);

                if (*gradient_type == GradientType::Asc && elev_diff >= 0.)
                    || (*gradient_type == GradientType::Desc && elev_diff < 0.)
                {
                    return p2_index;
                }

                p1_index
            };

        (1..telemetry.latlng.data.len()).for_each(|next_index| {
            let gradient = gradient_at(next_index);

            if !in_gradient {
                curr_gradient_type = get_gradient_type(gradient);

                if curr_gradient_type != GradientType::None {
                    in_gradient = true;

                    curr_gradient_start_index = next_index;
                    curr_gradient_end_index = next_index;
                }
            } else {
                if gradient_flipped(gradient, &curr_gradient_type) {
                    if distance_between(curr_gradient_end_index, next_index)
                        >= GradientFinder::DIST_GRADIENT_FLUCT_ALLOWANCE
                    {
                        let gradient_length =
                            distance_between(curr_gradient_start_index, curr_gradient_end_index);

                        if gradient_length > min_gradient_length_for_type(&curr_gradient_type) {
                            gradients.push(Gradient {
                                start: curr_gradient_start_index,
                                end: curr_gradient_end_index,
                                gradient: gradient_between(
                                    curr_gradient_start_index,
                                    curr_gradient_end_index,
                                ),
                            });

                            println!(
                                "Gradient between {:.1} {:.1} length {:.1}km gradient {:.2}",
                                distance_at(curr_gradient_start_index) / 1000.0,
                                distance_at(curr_gradient_end_index) / 1000.0,
                                gradient_length / 1000.0,
                                gradient_between(
                                    curr_gradient_start_index,
                                    curr_gradient_end_index
                                )
                            );
                        }

                        curr_gradient_type = GradientType::None;
                        curr_gradient_start_index = 0;
                        curr_gradient_end_index = 0;
                        in_gradient = false;
                    }
                } else {
                    curr_gradient_end_index = try_extend_gradient_with_point(
                        &curr_gradient_type,
                        curr_gradient_end_index,
                        next_index,
                    );
                }
            }
        });

        gradients
    }
}
