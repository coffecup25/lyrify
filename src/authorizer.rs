use crate::response::{GeniusAuth, Response, SpotifyAuth};
use base64;
use serde::Deserialize;
use std::{env, fs, marker::PhantomData};

pub struct Authorizer<T> {
    client_id: String,
    client_secret: String,
    scope: Vec<String>,
    redirect_uri: String,
    state: Option<String>,
    endpoints: Vec<String>,
    auth: Option<T>,
}

impl<T> Authorizer<T>
where
    T: Response + for<'de> Deserialize<'de>,
{
    pub fn authorize(&self) -> T {
        let client = reqwest::blocking::Client::new();
        let auth_url = self.auth_url(&client);

        open::that(auth_url).unwrap(); // open the url in the default browser so the user can sign in

        println!("Paste the url from the browser"); // todo: start server that listens on a local adress instead

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();

        let response_url = reqwest::Url::parse(&buf).unwrap();
        let mut queries = response_url.query_pairs();
        let (key, val) = queries.next().unwrap();
        match (*key).as_ref() {
            "code" => {}
            "error" => {
                panic!("{}", val)
            } // todo: return error instead
            _ => {
                panic!("response_url isnt correct")
            }
        }
        let auth_code = val;
        self.exchange_for_token(&client, &auth_code)
    }

    /// Sends the request to authorize the application.
    /// Returns the url for the API's authorization page that prompts the user to authorize the application.
    fn auth_url(&self, client: &reqwest::blocking::Client) -> String {
        let request_url = reqwest::Url::parse_with_params(
            &self.endpoints[0],
            &[
                ("response_type", "code"),
                ("client_id", &self.client_id),
                ("scope", &self.scope.join(",")),
                ("redirect_uri", &self.redirect_uri),
            ],
        )
        .unwrap();
        println!("request: {}", request_url);

        let res = client.get(request_url).send().unwrap();
        let url = res.url().to_string();
        url
    }

    fn exchange_for_token(&self, client: &reqwest::blocking::Client, auth_code: &str) -> T {
        let url = &self.endpoints[1];
        let params = [
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", &self.redirect_uri.to_string()),
            ("code", &auth_code.to_string()),
            ("grant_type", &"authorization_code".to_string()),
        ];
        let res = client.post(url).form(&params).send().unwrap();

        let auth_res: T = res.json().unwrap();
        auth_res
    }
}

impl Authorizer<SpotifyAuth> {
    fn with_client_id_secret(client_id: String, client_secret: String) -> Self {
        Authorizer::<SpotifyAuth> {
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
            auth: None,
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
    pub fn from_json_file(file_path: &String) -> SpotifyAuth {
        let config_file = fs::read_to_string(file_path).unwrap();
        let mut config_json: serde_json::Value =
            serde_json::from_str(&config_file).expect("config file couldn't be parsed as json");
        let mut spotify_json = config_json
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

        if let Some(refresh_token) = spotify_json.get("REFRESH_TOKEN") {
            let client = reqwest::blocking::Client::new();

            //Requesting for a new access token using the refresh token.
            let base64_encoded = base64::encode(format! {"{}:{}",client_id,client_secret});
            let refresh_token = refresh_token.to_string().replace('"', "");

            let body = reqwest::blocking::Body::from(
                format! {"grant_type=refresh_token&refresh_token={}",refresh_token.to_string()
                .replace('"', "")},
            );

            let response = client
                .post(&authorizer.endpoints[1])
                .header("Content-type", "application/x-www-form-urlencoded")
                .header("Authorization", format! {"Basic {}",base64_encoded})
                .body(body)
                .send()
                .unwrap();

            let mut res_json: serde_json::Value = serde_json::from_reader(response).unwrap();
            res_json["refresh_token"] = refresh_token.into();

            let auth = serde_json::from_value(res_json).unwrap();

            return auth;
        } else {
            let auth = authorizer.authorize();
            config_json["SPOTIFY"]["REFRESH_TOKEN"] =
                serde_json::to_value(auth.refresh_token()).unwrap();
            fs::write(file_path, config_json.to_string()).unwrap();
            return auth;
        }
    }
}

impl Authorizer<GeniusAuth> {
    fn with_client_id_secret(client_id: String, client_secret: String) -> Self {
        Authorizer::<GeniusAuth> {
            client_id,
            client_secret,
            scope: vec![],
            redirect_uri: "https://127.0.0.1:9090".to_string(),
            state: Some("adsa23134a".to_string()),
            endpoints: vec![
                "https://api.genius.com/oauth/authorize".to_string(),
                "https://api.genius.com/oauth/token".to_string(),
            ],
            auth: None,
        }
    }

    pub fn from_env() -> Self {
        let client_id =
            env::var("GENIUS_CLIENT_ID").expect("Set the GENIUS_CLIENT_ID env variable ");
        let client_secret =
            env::var("GENIUS_CLIENT_SECRET").expect("Set the GENIUS_CLIENT_SECRET env variable ");
        Self::with_client_id_secret(client_id, client_secret)
    }

    /// Reads the client id and secret from the supplied file. The json should be structured like this:
    /// GENIUS{CLIENT_ID:<>, CLIENT_SECRET: <>}
    pub fn from_json_file(file_path: &String) -> Self {
        let mut config_file = fs::File::open(file_path).unwrap();
        let config_json: serde_json::Value =
            serde_json::from_reader(config_file).expect("config file couldn't be parsed as json");
        let genius_json = config_json
            .get("GENIUS")
            .expect("No GENIUS object in the file");

        let client_id = genius_json
            .get("CLIENT_ID")
            .expect("Couldn't find CLIENT_ID attribute in the file");
        let client_secret = genius_json
            .get("CLIENT_SECRET")
            .expect("Couldn't find CLIENT_SECRET attribute in the file");

        Self::with_client_id_secret(
            client_id.to_string().replace('"', ""),
            client_secret.to_string().replace('"', ""),
        )
    }
}
