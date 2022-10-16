use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone)]
#[serde(into = "String")]
pub enum CourseGrade {
    Unfinished,

    Four = 4, //TODO investigate limits for getting credits
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Pass,
}

impl From<CourseGrade> for String {
    fn from(grade: CourseGrade) -> Self {
        match grade {
            CourseGrade::Unfinished => "T".to_string(),
            CourseGrade::Four => "4".to_string(),
            CourseGrade::Five => "5".to_string(),
            CourseGrade::Six => "6".to_string(),
            CourseGrade::Seven => "7".to_string(),
            CourseGrade::Eight => "8".to_string(),
            CourseGrade::Nine => "9".to_string(),
            CourseGrade::Ten => "10".to_string(),
            CourseGrade::Pass => "S".to_string(),
        }
    }
}

impl TryFrom<String> for CourseGrade {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "T" => Ok(CourseGrade::Unfinished),
            "4" => Ok(CourseGrade::Four),
            "5" => Ok(CourseGrade::Five),
            "6" => Ok(CourseGrade::Six),
            "7" => Ok(CourseGrade::Seven),
            "8" => Ok(CourseGrade::Eight),
            "9" => Ok(CourseGrade::Nine),
            "10" => Ok(CourseGrade::Ten),
            "S" => Ok(CourseGrade::Pass),
            _ => Err(anyhow!("Invalid grade")),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Course {
    pub code: String,
    pub name: String,
    pub selected: bool,
    pub selectable: bool,
    pub optional: bool,
    #[serde(rename = "Type")]
    pub type_: String,
    pub grade: Option<CourseGrade>,
    pub completed_at: Option<String>, //TODO: Use real date?

    pub study_weeks: f32,
    pub study_points: f32,
}

impl Course {
    pub fn eligible_for_points(&self) -> bool {
        matches!(
            self.grade,
            Some(CourseGrade::Four) //TODO investigate limits for getting credits
                | Some(CourseGrade::Five)
                | Some(CourseGrade::Six)
                | Some(CourseGrade::Seven)
                | Some(CourseGrade::Eight)
                | Some(CourseGrade::Nine)
                | Some(CourseGrade::Ten)
                | Some(CourseGrade::Pass)
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(from = "String")]
pub enum WilmaRoleType {
    Passwd,
    Student,
    Teacher,

    Unknown = -1,
}

impl From<String> for WilmaRoleType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "passwd" => Self::Passwd,
            "student" => Self::Student,
            "teacher" => Self::Teacher,
            _ => Self::Unknown,
        }
    }
}

#[derive(Deserialize)]
pub struct WilmaRoleResponse {
    pub payload: Vec<WilmaRole>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WilmaRole {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: WilmaRoleType,
    #[serde(rename = "primusId")]
    pub primus_id: i32,
    #[serde(rename = "formKey")]
    pub form_key: String,
    pub slug: String,
    pub schools: Vec<WilmaSchool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WilmaSchool {
    pub id: i32,
    pub caption: String,
}

#[derive(Deserialize, Debug)]
pub struct WilmaIndexJson {
    #[serde(rename = "LoginResult")]
    pub login_result: String,
    #[serde(rename = "SessionID")]
    pub session_id: String,
    #[serde(rename = "ApiVersion")]
    pub api_version: i32,
    pub oidc_test_mode: Option<bool>,
    pub oidc_providers: Option<Vec<OpenIDProvider>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OpenIDProvider {
    pub name: String,
    pub client_id: String,
    pub configuration: String,
    pub scope: String,
}

#[derive(Deserialize, Debug)]
pub struct OpenIDConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

#[derive(Deserialize, Debug)]
pub struct WilmaHubWilma {
    pub url: String,
    pub name: String,
}
