use reqwest::Client;
use scraper::{ElementRef, Html, Node, Selector};

use anyhow::{anyhow, ensure, Result};

use serde::Deserialize;
use serde_json::from_str;

use lazy_static::lazy_static;
use regex::Regex;

use crate::wilma::Wilma;

use super::models::{Course, CourseGrade};

lazy_static! {
    static ref COMPULSORY_REGEX: Regex = Regex::new(r"choicesCompulsoryTypes = (\[.*\]);").unwrap();
    static ref SELECTABLE_REGEX: Regex = Regex::new(r"choicesSelectableTypes = (\[.*\]);").unwrap();
    static ref COURSE_CLASS_REGEX: Regex = Regex::new(r"c-type(\d+)-?(sel|graded)?").unwrap();
    static ref COURSE_ROOT_SELECTOR: Selector =
        Selector::parse("#choices-tree > li > ul > li a").unwrap();
    static ref COURSE_SELECTOR: Selector = Selector::parse("ul > li").unwrap();
    static ref A_SELECTOR: Selector = Selector::parse("a").unwrap();
    static ref GRADE_SELECTOR: Selector = Selector::parse("td").unwrap();
}

#[derive(Deserialize)]
struct CourseData {
    #[serde(rename = "Lyhenne")]
    code: String,
    #[serde(rename = "Nimi")]
    name: String,
    #[serde(rename = "Kurssityyppi")]
    type_: String,
    ov: Option<String>,
    op: Option<String>,
    #[serde(rename = "Suorituspvm.")]
    completed_at: Option<String>,
}

fn parse_course(element: ElementRef, compulsory: &[i32], selectable: &[i32]) -> Option<Course> {
    if element.value().name() != "a" {
        return None;
    }

    let class = element.value().attr("class")?;
    let data = element.value().attr("data-jsontitle")?;
    let data: CourseData = from_str(data).ok()?;

    //possible ul element is always last
    let is_bottom_level = element.next_siblings().all(|c| match c.value() {
        Node::Element(e) => e.name() == "a",
        _ => false,
    });
    if !is_bottom_level {
        return None;
    }

    let grade = CourseGrade::try_from(element.select(&GRADE_SELECTOR).next()?.inner_html()).ok();

    let captures = COURSE_CLASS_REGEX.captures(class)?;
    let type_: i32 = captures.get(1)?.as_str().parse().ok()?;

    let is_compulsory = compulsory.contains(&type_);
    let is_selectable = selectable.contains(&type_);

    let is_selected = captures.get(2).map_or("", |m| m.as_str()) == "sel";
    let is_graded = captures.get(2).map_or("", |m| m.as_str()) == "graded";

    let ov = data
        .ov
        .and_then(|s| s.strip_suffix("ov").map(|v| v.to_string()))
        .and_then(|s| s.parse::<f32>().ok());
    let op = data
        .op
        .and_then(|s| s.strip_suffix("op").map(|v| v.to_string()))
        .and_then(|s| s.parse::<f32>().ok());

    let (ov, op) = if let Some(n) = ov {
        (Some(n), Some(n * 2.0))
    } else if let Some(n) = op {
        (Some(n / 2.0), Some(n))
    } else {
        (None, None)
    };

    Some(Course {
        code: data.code,
        name: data.name,
        type_: data.type_,
        selected: is_selected || is_graded,
        selectable: is_selectable,
        completed_at: data.completed_at,
        optional: !is_compulsory,
        grade: if is_graded { grade } else { None },
        study_points: op?,
        study_weeks: ov?,
    })
}

pub async fn get_courses(client: &Client, wilma: &Wilma) -> Result<Vec<Course>> {
    ensure!(wilma.is_logged_in(), "Not logged in");

    let html = client
        .get(wilma.get_url()?.join("choices?langid=1")?)
        .header(
            "Cookie",
            format!("Wilma2SID={};", wilma.sid.as_ref().unwrap()),
        )
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(html.as_str());

    let compulsory: Vec<i32> = from_str(
        COMPULSORY_REGEX
            .captures(html.as_str())
            .ok_or_else(|| anyhow!("Could not find compulsory ids"))?
            .get(1)
            .ok_or_else(|| anyhow!("Could not find compulsory ids"))?
            .as_str(),
    )?;
    let selectable: Vec<i32> = from_str(
        SELECTABLE_REGEX
            .captures(html.as_str())
            .ok_or_else(|| anyhow!("Could not find selectable ids"))?
            .get(1)
            .ok_or_else(|| anyhow!("Could not find selectable ids"))?
            .as_str(),
    )?;

    let courses = document
        .select(&COURSE_ROOT_SELECTOR)
        .filter_map(|e| parse_course(e, &compulsory, &selectable))
        .collect::<Vec<Course>>();

    Ok(courses)
}
