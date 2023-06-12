pub mod v0;

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Eq)]
pub enum OrderBy {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

pub trait IndexLike {}

pub trait IndexPropertyLike {}
