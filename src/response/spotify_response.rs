use serde::Deserialize;

use super::AccessTokenQuery;

#[derive(Deserialize, Debug)]
pub struct SpotifyClient {
    //taken from https://developer.spotify.com/documentation/general/guides/authorization-guide/
    ///An access token that can be provided in subsequent calls, for example to Spotify Web API services.
    pub access_token: String,

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

impl SpotifyClient {
    pub fn get_refresh_token(&self) -> &String {
        &self.refresh_token
    }

    pub fn get_song_playing(&self, client: &reqwest::blocking::Client) -> Result<SpotifyHit, String>{
        let json_res = self.send_request("https://api.spotify.com/v1/me/player/currently-playing", client)?;

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
