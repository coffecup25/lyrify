use serde::Deserialize;
use std::{error::Error, marker::PhantomData};

fn main() -> Result<(), Box<dyn Error>> {
    let spotify_auth = Authorizer::<SpotifyAuthResponse>::default();
    let genius_auth = Authorizer::<GeniusAuthResponse>::default();

    let spotify_auth_response = spotify_auth.authorize();
    let genius_auth_response = genius_auth.authorize();
    println!("Program ready");
    spotify_auth_response.query("https://api.spotify.com/v1/me/player/currently-playing");
    genius_auth_response.query("https://api.genius.com/songs/378195");

    Ok(())
}
#[derive(Deserialize, Debug)]
struct GeniusAuthResponse {
    access_token: String,
}
impl GeniusAuthResponse {
    pub fn query(&self, url: &str) {
        let client = reqwest::blocking::Client::new();
        let res = client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .unwrap();
        println!("#############\n{}", res.text().unwrap());
    }
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

impl SpotifyAuthResponse {
    pub fn query(&self, url: &str) {
        let client = reqwest::blocking::Client::new();
        let res = client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .unwrap();
        println!("#############\n{}", res.text().unwrap());
    }
}

impl Response for SpotifyAuthResponse {}
impl Response for GeniusAuthResponse {}

trait Response {}

struct Authorizer<'a, T> {
    client_id: &'a str,
    client_secret: &'a str,
    scope: Vec<&'a str>,
    redirect_uri: &'a str,
    state: Option<&'a str>,
    endpoints: Vec<&'a str>,
    phantom: PhantomData<T>,
}

impl<'a, T> Authorizer<'a, T>
where
    T: Response + for<'de> Deserialize<'de>,
{
    pub fn authorize(&self) -> T {
        let auth_url = self.auth_url();

        open::that(auth_url); // open the url in the default browser so the user can sign in

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
            self.endpoints[0],
            &[
                ("response_type", "code"),
                ("client_id", self.client_id),
                ("scope", &self.scope.join(",")),
                ("redirect_uri", self.redirect_uri),
            ],
        )
        .unwrap();
        println!("request: {}", request_url);

        let res = reqwest::blocking::get(request_url).unwrap();
        let url = res.url().to_string();
        url.to_string()
    }

    fn exchange_for_token(&self, auth_code: &str) -> T {
        let url = self.endpoints[1];
        let client = reqwest::blocking::Client::new();
        let params = [
            ("client_id", self.client_id),
            ("client_secret", self.client_secret),
            ("redirect_uri", self.redirect_uri),
            ("code", auth_code),
            ("grant_type", "authorization_code"),
        ];
        let res = client.post(url).form(&params).send().unwrap();
        let auth_res: T = res.json().unwrap();
        auth_res
    }
}

impl <'a> Authorizer<'a,SpotifyAuthResponse>{
    fn default() -> Self{
        Authorizer::<SpotifyAuthResponse> {
            client_id:
            client_secret: 
            scope: vec![
                "user-read-email",
                "user-read-private",
                "user-read-currently-playing",
                "user-modify-playback-state",
            ],
            redirect_uri: "http://127.0.0.1:9090",
            state: None,
            endpoints: vec![
                "https://accounts.spotify.com/authorize?",
                "https://accounts.spotify.com/api/token",
            ],
            phantom: PhantomData,
        }
    }
}

impl <'a> Authorizer<'a,GeniusAuthResponse>{
    fn default()->Self{
        Authorizer::<GeniusAuthResponse> {
            client_id: "vrqTGne_jL9k_pMF_k89GQWExtdMlbeBR0pOwAEZF4fEfVJ-3v4J8mO60N84VSRR",
            client_secret:
                "iG2QW3uW2BoTsaEaRlATzOoJJdxm0U9QPoAqS56g12EsE11A6habGqM1ORIuKqa5rbaitD5bSRYroCkGWR6usA",
            scope: vec!["me"],
            redirect_uri: "https://127.0.0.1:9090",
            state: Some("adsa23134a"),
            endpoints: vec![
                "https://api.genius.com/oauth/authorize",
                "https://api.genius.com/oauth/token",
            ],
            phantom: PhantomData,
        }
    }
}