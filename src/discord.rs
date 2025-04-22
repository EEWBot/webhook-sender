use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ratelimit {
    pub retry_after: f32,
}
