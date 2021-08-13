use serde::Deserialize;
use std::{error::Error,env, io, marker::PhantomData};

fn main() -> Result<(), Box<dyn Error>> {
    let spotify_auth = Authorizer::<SpotifyAuthResponse>::from_env();
    let spotify_auth_response = spotify_auth.authorize();
    let json_response=spotify_auth_response.query("https://api.spotify.com/v1/me/player/currently-playing").unwrap();
    // match spotify_auth_response.query("https://api.spotify.com/v1/me/player/currently-playing"){
    //     Ok(json_result) =>{   
    //         println!("The currently playing track is: {} by {}",json_result["item"]["name"],json_result["item"]["artists"][0]["name"] );
    //     },
    //     Err(s) =>println!("{}",s),
    // }
    let song = &json_response["item"]["name"];
    let artist = &json_response["item"]["artists"][0]["name"];
    println!("The currently playing track is: {} by {}",song,artist);


    let genius_auth = Authorizer::<GeniusAuthResponse>::from_env();
    let genius_auth_response = genius_auth.authorize();
    let query_url = reqwest::Url::parse_with_params(
        "https://api.genius.com/search",
        &[
            ("q", format!("{} {}",song,artist)),
    
        ],
    ).unwrap();
    let json_response = match genius_auth_response.query(&query_url.to_string()){
        Ok(json_result) =>{   
            json_result
        },
        Err(s) =>panic!("{}",s),
    };

    let song =&json_response["response"]["hits"][0]["result"]["id"];
    let lyric_path = &json_response["response"]["hits"][0]["result"]["path"];

    
    
    let query_url = reqwest::Url::parse_with_params(
        &format!("https://api.genius.com/songs/{}",song),
        &[
            ("text_format", "plain"),
    
        ],
    ).unwrap();
    let json_response = match genius_auth_response.query(&query_url.to_string()){
        Ok(json_result) =>{   
            json_result
        },
        Err(s) =>panic!("{}",s),
    };

    Ok(())
}



#[derive(Deserialize, Debug)]
struct GeniusAuthResponse {
    access_token: String,
}

#[derive(Deserialize, Debug)]
struct SpotifyAuthResponse {
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
    //(When the access code expires, send a POST request to the Accounts service /api/token endpoint
    ///but use this code in place of an authorization code. A new access token will be returned.
    // A new refresh token might be returned too.)
    refresh_token: String,
}


impl Response for SpotifyAuthResponse {
    fn access_token(&self) -> &String {
        &self.access_token
    }
}
impl Response for GeniusAuthResponse {
    fn access_token(&self) -> &String {
        &self.access_token
    }
}

trait Response {
    
       fn query(&self, url: &str) -> Result<serde_json::Value,String> {
            let client = reqwest::blocking::Client::new();
            let res = client
                .get(url)
                .bearer_auth(&self.access_token())
                .send()
                .unwrap();
            match res.status() {
                reqwest::StatusCode::NO_CONTENT=>{return Err("No song playing".to_string());}
                _ =>{}
            }
            let result = res.text().unwrap();
            std::fs::write("response.json", &result).expect("lord we fucked up");
            Ok(serde_json::from_str(&result).unwrap())
        }

        fn access_token(&self) -> &String;
    
}

struct Authorizer<T> {
    client_id: String,
    client_secret: String,
    scope: Vec<String>,
    redirect_uri: String,
    state: Option<String>,
    endpoints: Vec<String>,
    phantom: PhantomData<T>,
}

impl<T> Authorizer <T>
where
    T: Response + for<'de> Deserialize<'de>,
{
    pub fn authorize(&self) -> T {
        let auth_url = self.auth_url();

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
        self.exchange_for_token(&auth_code)
    }

    fn auth_url(&self) -> String {
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

        let res = reqwest::blocking::get(request_url).unwrap();
        let url = res.url().to_string();
        url.to_string()
    }

    fn exchange_for_token(&self, auth_code: &str) -> T {
        let url = &self.endpoints[1];
        let client = reqwest::blocking::Client::new();
        let params = [
            ("client_id", &self.client_id),
            ("client_secret",&self.client_secret),
            ("redirect_uri", &self.redirect_uri.to_string()),
            ("code", &auth_code.to_string()),
            ("grant_type", &"authorization_code".to_string()),
        ];
        let res = client.post(url).form(&params).send().unwrap();
        let auth_res: T = res.json().unwrap();
        auth_res
    }
}

impl Authorizer <SpotifyAuthResponse>{
    fn from_env() -> Self{
        let client_id = env::var("SPOTIFY_CLIENT_ID").expect("Set the SPOTIFY_CLIENT_ID env variable ");
        let client_secret=  env::var("SPOTIFY_CLIENT_SECRET").expect("Set the SPOTIFY_CLIENT_SECRET env variable ");
        Authorizer::<SpotifyAuthResponse> {
            client_id: client_id,
            client_secret: client_secret,
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
            phantom: PhantomData,
        }
    }
}

impl Authorizer<GeniusAuthResponse>{
    fn from_env() -> Self{
        let client_id = env::var("GENIUS_CLIENT_ID").expect("Set the GENIUS_CLIENT_ID env variable ");
        let client_secret=  env::var("GENIUS_CLIENT_SECRET").expect("Set the GENIUS_CLIENT_SECRET env variable ");
        Authorizer::<GeniusAuthResponse> {
            client_id,
            client_secret,
            scope: vec!["me".to_string()],
            redirect_uri: "https://127.0.0.1:9090".to_string(),
            state: Some("adsa23134a".to_string()),
            endpoints: vec![
                "https://api.genius.com/oauth/authorize".to_string(),
                "https://api.genius.com/oauth/token".to_string(),
            ],
            phantom: PhantomData,
        }
    }
}