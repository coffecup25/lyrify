use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GeniusAuth {
    access_token: String,
}

impl GeniusAuth {
    pub fn from_access_token(access_token: String) -> Self {
        GeniusAuth { access_token }
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
}

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

pub struct GeniusHit {
    pub artist: String,
    pub lyric_path: String,
}

impl GeniusHit {
    pub fn new(artist: String, lyric_path: String) -> Self {
        GeniusHit { artist, lyric_path }
    }
}

#[derive(Deserialize, Debug)]
pub struct SpotifyAuth {
    //taken from https://developer.spotify.com/documentation/general/guides/authorization-guide/
    ///An access token that can be provided in subsequent calls, for example to Spotify Web API services.
    access_token: String,

    /// How the access token may be used: always “Bearer”.
    token_type: String,

    /// A space-separated list of scopes which have been granted for this access_token
    scope: String,

    /// The time period (in seconds) for which the access token is valid.
    expires_in: usize,

    ///A token that can be sent to the Spotify Accounts service in place of an authorization code.
    ///(When the access code expires, send a POST request to the Accounts service /api/token endpoint
    ///but use this code in place of an authorization code. A new access token will be returned.
    /// A new refresh token might be returned too.)
    refresh_token: String,
}

pub struct SpotifyClient {
    auth: SpotifyAuth,
}

struct SpotifyQuery<'a> {
    client: &'a SpotifyClient,
}

impl SpotifyClient {
    pub fn get_currently_playing_song(&self, client: &reqwest::blocking::Client) {
        let spotify_response = self
            .auth
            .query(
                "https://api.spotify.com/v1/me/player/currently-playing",
                client,
            )
            .unwrap();

        let mut song = spotify_response.title;
        let spotify_artist = &spotify_response.artist;
    }
    fn user(&self) {}
}

pub trait Response {
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

impl SpotifyAuth {
    pub fn refresh_token(&self) -> &String {
        &self.refresh_token
    }
    pub fn query(
        &self,
        url: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<SpotifyHit, String> {
        let json_res = self.send_request(url, client)?;

        let song_title = json_res["item"]["name"]
            .as_str()
            .ok_or_else(|| "no song found in spotify response")?;
        let spotify_artist = json_res["item"]["artists"][0]["name"]
            .as_str()
            .ok_or_else(|| "no artist found in spotify response")?;

        #[cfg(feature = "debug")]
        std::fs::write("response_spotify.json", &json_res.to_string()).expect("lord we fucked up");

        Ok(SpotifyHit::new(
            song_title.to_owned(),
            spotify_artist.to_owned(),
        ))
    }
}

impl Response for SpotifyAuth {
    fn access_token(&self) -> &String {
        &self.access_token
    }
}
impl Response for GeniusAuth {
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
