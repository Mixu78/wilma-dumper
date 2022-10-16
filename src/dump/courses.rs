use anyhow::Result;
use std::io::Write;

use crate::wilma::models::Course;

#[derive(Clone, PartialEq)]
pub enum Format {
    Json,
    Csv,
}

impl ToString for Format {
    fn to_string(&self) -> String {
        match self {
            Format::Json => "json".to_string(),
            Format::Csv => "csv".to_string(),
        }
    }
}

pub fn dump_to_writer(courses: &Vec<Course>, writer: impl Write, format: Format) -> Result<()> {
    match format {
        Format::Json => {
            serde_json::to_writer(writer, courses)?;
        }
        Format::Csv => {
            let mut csv = csv::Writer::from_writer(writer);
            for c in courses {
                csv.serialize(c).unwrap();
            }
        }
    };

    Ok(())
}

pub fn calculate_study_points(courses: &[Course]) -> (f32, f32) {
    let selected = courses.iter().fold(0.0, |acc, c| match c.selected {
        true => acc + c.study_points,
        false => acc,
    });

    let earned = courses
        .iter()
        .fold(0.0, |acc, c| match c.eligible_for_points() {
            true => acc + c.study_points,
            false => acc,
        });

    (selected, earned)
}
