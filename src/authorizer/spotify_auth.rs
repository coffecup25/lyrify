use crate::Authorizer;
use crate::response::spotify_response::SpotifyClient;

use base64;
use serde_json::Value;
use std::{env, fs, marker::PhantomData};


impl Authorizer<SpotifyClient> {
    fn with_client_id_secret(client_id: String, client_secret: String) -> Self {
        Authorizer::<SpotifyClient> {
            client_id,
            client_secret,
            scope: vec![
                "user-read-email".to_string(),
                "user-read-private".to_string(),
                "user-read-currently-playing".to_string(),
                "user-modify-playback-state".to_string(),
            ],
            redirect_uri: "http://127.0.0.1:9090".to_string(),
            state: None,
            endpoints: vec![
                "https://accounts.spotify.com/authorize".to_string(),
                "https://accounts.spotify.com/api/token".to_string(),
            ],
            phantom: PhantomData::<SpotifyClient>,
        }
    }
    /// Gets the client_id and client_secret from the SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET respectively
    pub fn from_env() -> Self {
        let client_id =
            env::var("SPOTIFY_CLIENT_ID").expect("Set the SPOTIFY_CLIENT_ID env variable ");
        let client_secret =
            env::var("SPOTIFY_CLIENT_SECRET").expect("Set the SPOTIFY_CLIENT_SECRET env variable ");

        Self::with_client_id_secret(client_id, client_secret)
    }
    /// Reads the client id and secret from the supplied file. The json should be structured like this:
    /// SPOTIFY{CLIENT_ID:<>, CLIENT_SECRET: <>}
    pub fn from_json_file(file_path: &str) -> SpotifyClient {
        let config_file = fs::read_to_string(file_path).unwrap();
        let mut config_json: serde_json::Value =
            serde_json::from_str(&config_file).expect("config file couldn't be parsed as json");

        let spotify_json = config_json
            .get("SPOTIFY")
            .expect("No spotify object in the file");

        let client_id = spotify_json
            .get("CLIENT_ID")
            .expect("Couldn't find CLIENT_ID attribute in the file")
            .to_string()
            .replace('"', "");
        let client_secret = spotify_json
            .get("CLIENT_SECRET")
            .expect("Couldn't find CLIENT_SECRET attribute in the file")
            .to_string()
            .replace('"', "");

        let authorizer = Self::with_client_id_secret(client_id.clone(), client_secret.clone());
        
        // if an refresh_token is saved we use it to get the access token instead of authorizing again.
        if let Some(refresh_token) = spotify_json.get("REFRESH_TOKEN") {
            let auth =  Self::get_authorized_with_refresh_token(&authorizer,refresh_token, &client_id, &client_secret);
            if let Ok(spotify_auth) = auth{
                return spotify_auth;
            }
        }

        let auth = authorizer.authorize();

        // Saving the refresh token to the config file.
        config_json["SPOTIFY"]["REFRESH_TOKEN"] =
            serde_json::to_value(auth.get_refresh_token()).unwrap();
        fs::write(file_path, config_json.to_string()).unwrap();

        auth
    }

    fn get_authorized_with_refresh_token(authorizer: &Authorizer<SpotifyClient>, refresh_token: &Value, client_id: &str, client_secret: &str) -> Result<SpotifyClient, String>{
        let client = reqwest::blocking::Client::new();
        //Requesting for a new access token using the refresh token.
        let base64_encoded = base64::encode(format! {"{}:{}",client_id,client_secret});
        let refresh_token = refresh_token.to_string().replace('"', "");

        let body = reqwest::blocking::Body::from(
            format! {"grant_type=refresh_token&refresh_token={}",refresh_token},
        );

        let response = client
            .post(&authorizer.endpoints[1])
            .header("Content-type", "application/x-www-form-urlencoded")
            .header("Authorization", format! {"Basic {}",base64_encoded})
            .body(body)
            .send()
            .unwrap();

        if response.status().is_success() {
            let mut res_json: serde_json::Value = serde_json::from_reader(response).unwrap();
            res_json["refresh_token"] = refresh_token.into();

            let auth = serde_json::from_value(res_json).unwrap();
            return Ok(auth);
        }

        Err("Couldn't get access token with refresh token".to_string())
    }
}