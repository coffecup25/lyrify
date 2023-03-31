pub mod spotify_response;
pub(crate) mod genius_response;

use self::{genius_response::GeniusClient, spotify_response::SpotifyClient};

/// Trait that contains the functionality for querying with an access token.
///
/// It is used to enable both the Spotify and Genius clients to query with their respective access tokens.

pub trait AccessTokenQuery {
    fn send_request(
        &self,
        url: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<serde_json::Value, String> {
        let res = client
            .get(url)
            .bearer_auth(&self.access_token())
            .send()
            .unwrap();

        if res.status() == reqwest::StatusCode::NO_CONTENT {
            return Err("No song playing".to_string());
        }

        let result = res.text().unwrap();
        std::fs::write("response.json", &result).expect("lord we fucked up");
        Ok(serde_json::from_str(&result).unwrap())
    }

    fn access_token(&self) -> &String;
}

impl AccessTokenQuery for SpotifyClient {
    fn access_token(&self) -> &String {
        &self.access_token
    }
}
impl AccessTokenQuery for GeniusClient {
    fn access_token(&self) -> &String {
        &self.access_token
    }
}

#[derive(Debug)]
pub struct SpotifyHit {
    pub title: String,
    pub artist: String,
}
impl SpotifyHit {
    pub fn new(title: String, artist: String) -> SpotifyHit {
        SpotifyHit { title, artist }
    }
}
