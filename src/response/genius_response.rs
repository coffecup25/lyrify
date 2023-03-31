use serde::Deserialize;

use super::AccessTokenQuery;

pub struct GeniusHits {
    pub hits: Vec<serde_json::Value>,
    counter: usize,
}

impl GeniusHits {
    pub fn new(hits: Vec<serde_json::Value>) -> Self {
        GeniusHits { hits, counter: 0 }
    }
}

struct Hit {
    song: String,
    artist: String,
    lyrics_path: String,
    song_art_path: String,
}

impl Iterator for GeniusHits {
    type Item = GeniusHit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter >= self.hits.len() {
            return None;
        }

        let lyric_path = self.hits[self.counter]["result"]["path"].as_str().unwrap();
        let artist = self.hits[self.counter]["result"]["primary_artist"]["name"]
            .as_str()
            .unwrap()
            .replace("\"", "");

        self.counter += 1;
        Some(GeniusHit::new(
            artist.trim().to_string(),
            lyric_path.to_string(),
        ))
    }
}

/// A Genius hit is a single search result, and contains the title of the song, the artist, and the URL of the song's page on Genius.com.
pub struct GeniusHit {
    /// The artist of the song
    pub artist: String,
    /// The path to the lyrics page on Genius.com
    pub lyric_path: String,
}

impl GeniusHit {
    pub fn new(artist: String, lyric_path: String) -> Self {
        GeniusHit { artist, lyric_path }
    }
}

#[derive(Deserialize, Debug)]
pub struct GeniusClient {
    pub access_token: String,
}

impl GeniusClient {
    pub fn from_access_token(access_token: String) -> Self {
        GeniusClient { access_token }
    }

    pub fn query(
        &self,
        url: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<GeniusHits, String> {
        let json_res = self.send_request(url, client)?;
        let hits = json_res["response"]["hits"]
            .as_array()
            .ok_or_else(|| "no genius hits")?;

        #[cfg(feature = "debug")]
        std::fs::write("response_genius.json", &json_res.to_string()).expect("lord we fucked up");

        Ok(GeniusHits::new(hits.to_owned()))
    }

    pub fn artist_and_song_search(&self, song: &String, artist: &String, client: &reqwest::blocking::Client) -> Result<GeniusHits, String> {
        let mut query_url = reqwest::Url::parse_with_params(
            "https://api.genius.com/search",
            &[("q", format!("{} {}", song, artist))],
        )
        .unwrap();
    
        #[cfg(feature = "debug")]
        println!("genius query url: {}", query_url);
    
        self.query(&query_url.to_string(), client)
    }
}
