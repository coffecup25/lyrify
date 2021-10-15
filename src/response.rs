use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GeniusAuth {
    access_token: String,
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

pub trait Response {
    fn query(&self, url: &str,client:  &reqwest::blocking::Client) -> Result<serde_json::Value, String> {
        let res = client
            .get(url)
            .bearer_auth(&self.access_token())
            .send()
            .unwrap();

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => {
                return Err("No song playing".to_string());
            }
            _ => {}
        }
        let result = res.text().unwrap();
        std::fs::write("response.json", &result).expect("lord we fucked up");
        Ok(serde_json::from_str(&result).unwrap())
    }
    fn access_token(&self) -> &String;
}
