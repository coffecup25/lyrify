use crate::response::genius_response::GeniusClient;
use crate::Authorizer;
use std::{env, fs, marker::PhantomData};

impl Authorizer<GeniusClient> {
    fn with_client_id_secret(client_id: String, client_secret: String) -> Self {
        Authorizer::<GeniusClient> {
            client_id,
            client_secret,
            scope: vec![],
            redirect_uri: "https://127.0.0.1:9090".to_string(),
            state: Some("adsa23134a".to_string()),
            endpoints: vec![
                "https://api.genius.com/oauth/authorize".to_string(),
                "https://api.genius.com/oauth/token".to_string(),
            ],
            phantom: PhantomData::<GeniusClient>,
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
    pub fn from_json_file(file_path: &str) -> GeniusClient {
        let config_file = fs::File::open(file_path).unwrap();
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
        let access_token = genius_json
            .get("ACCESS_TOKEN")
            .expect("Couldn't find ACCESS_TOKEN attribute in the file")
            .to_string()
            .replace('"', "");

        GeniusClient::from_access_token(access_token)
    }
}