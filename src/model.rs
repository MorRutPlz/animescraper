use serde::Deserialize;

#[derive(Deserialize)]
pub struct Animix {
    pub title: String,
    pub id: String,
    pub e: String,
}

#[derive(Deserialize)]
pub struct GogoStream {
    pub source: Option<GogoStreamSource>,
    pub source_bk: Option<GogoStreamSource>,
}

#[derive(Deserialize)]
pub struct GogoStreamSource {
    pub file: String,
}
