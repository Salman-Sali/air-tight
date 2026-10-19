/// `GET /health`: open to everyone, no authentication.
#[air_tight::unrestricted]
pub async fn health() -> &'static str {
    "ok"
}
