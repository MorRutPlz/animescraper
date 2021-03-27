use serde::Deserialize;

#[derive(Deserialize)]
pub struct Animix {
    pub title: String,
    pub id: String,
    pub e: String,
}
