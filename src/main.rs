use serde::Deserialize;
use soup::prelude::*;
use std::{
    env,
    error::Error,
    io::{self, Stderr},
    marker::PhantomData,
    str::FromStr,
};

fn main() -> Result<(), Box<dyn Error>> {
    let spotify_auth = Authorizer::<SpotifyAuthResponse>::from_env();
    let spotify_auth_response = spotify_auth.authorize();

    let genius_auth = Authorizer::<GeniusAuthResponse>::from_env();
    let genius_auth_response = genius_auth.authorize();

    // this cookie is super important. without it genius might return one of two different page layouts for the lyrics which makes scraping much harder. The page layout changes att the cookie value 50.
    let cookie_jar = reqwest::cookie::Jar::default();
    cookie_jar.add_cookie_str(
        "_genius_ab_test_cohort=80",
        &"https://genius.com".parse::<reqwest::Url>().unwrap(),
    );

    let client = reqwest::blocking::Client::builder()
        .cookie_provider(cookie_jar.into())
        .build()
        .unwrap();

    println!("Ready for queries");
    let mut buf = String::new();
    let input = io::stdin();
    loop {
        println!("Hit enter to get lyrics");
        input.read_line(&mut buf).unwrap();
        match get_lyrics(&client, &spotify_auth_response, &genius_auth_response) {
            Ok(lyrics) => println!("###########################\n{}\n", lyrics.trim_end()),
            Err(_) => println!("Couldn't find lyrics"),
        }
    }

    Ok(())
}

fn get_lyrics(
    client: &reqwest::blocking::Client,
    spotify_auth: &SpotifyAuthResponse,
    genius_auth: &GeniusAuthResponse,
) -> Result<String, ()> {
    let spotify_response = spotify_auth
        .query("https://api.spotify.com/v1/me/player/currently-playing")
        .unwrap();

    let mut song = spotify_response["item"]["name"].to_string();
    let artist = &spotify_response["item"]["artists"][0]["name"];

    #[cfg(feature = "debug")]
    std::fs::write("response_spotify.json", &spotify_response.to_string())
        .expect("lord we fucked up");

    println!("The currently playing track is: {} by {}", song, artist);

    song = remove_feat(&mut song);

    let query_url = reqwest::Url::parse_with_params(
        "https://api.genius.com/search",
        &[("q", format!("{} {}", &song, artist))],
    )
    .unwrap();

    let genius_response = match genius_auth.query(&query_url.to_string()) {
        Ok(json_result) => json_result,
        Err(s) => panic!("{}", s),
    };

    #[cfg(feature = "debug")]
    std::fs::write("response_genius.json", &genius_response.to_string())
        .expect("lord we fucked up");

    let _song = &genius_response["response"]["hits"][0]["result"]["id"];
    let lyric_path = &genius_response["response"]["hits"][0]["result"]["path"];

    let query_url = reqwest::Url::from_str(&format!(
        "https://genius.com{}",
        lyric_path.as_str().unwrap()
    ))
    .unwrap();
    println!("{}", &query_url);

    let response = client.get(query_url).send().unwrap();

    #[cfg(feature = "debug")]
    println!(
        "############# Response #############\n {:?}",
        response.headers()
    );

    let res = response.text().unwrap();

    //println!("\n\n{:#?}",response.text().unwrap());

    #[cfg(feature = "debug")]
    std::fs::write("response_final.html", &res.as_bytes()).expect("lord we fucked up");

    //tag("div").attr("class", "lyrics")
    let document = soup::Soup::new(res.as_str());
    match document.tag("div").class("lyrics").find() {
        Some(n) => Ok(n.text()),
        None => Err(()),
    }
}

struct GeniusHits {
    hits: serde_json::Value,
    counter: usize,
}

struct Hit {
    song: String,
    artist: String,
    lyrics_path: String,
    song_art_path: String,
}

impl Iterator for GeniusHits {
    type Item = serde_json::Value;

    fn next(&mut self) -> Option<Self::Item> {
        let song = &self.hits[self.counter]["result"]["id"];
        let lyric_path = &self.hits[self.counter]["result"]["path"];
        todo!()
    }
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
    fn query(&self, url: &str) -> Result<serde_json::Value, String> {
        let client = reqwest::blocking::Client::new();
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

#[derive(Clone, Copy)]
struct Encloser(char, char);

fn remove_feat(name: &mut String) -> String {
    let enclosers = [Encloser('(', ')'), Encloser('[', ']')];
    let mut enumerator = name.split_whitespace();
    let mut new_string = String::with_capacity(name.len());

    while let Some(word) = enumerator.next() {
        let mut word_iter = word.chars();
        if let Some(c) = word_iter.next() {
            // first character of word should be an encloser
            if let Some(encloser) = enclosers.iter().find(|&x| c == x.0) {
                if word_iter.as_str().to_lowercase() == "feat." {
                    while let Some(w) = enumerator.next() {
                        if w.chars().last().unwrap() == encloser.1 {
                            break;
                        }
                    }
                } else {
                    new_string.push_str(word);
                    new_string.push_str(" ");
                }
            } else {
                new_string.push_str(word);
                new_string.push_str(" ");
            }
        }
    }
    new_string = new_string.trim().to_string();
    new_string
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

impl Authorizer<SpotifyAuthResponse> {
    fn from_env() -> Self {
        let client_id =
            env::var("SPOTIFY_CLIENT_ID").expect("Set the SPOTIFY_CLIENT_ID env variable ");
        let client_secret =
            env::var("SPOTIFY_CLIENT_SECRET").expect("Set the SPOTIFY_CLIENT_SECRET env variable ");
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

impl Authorizer<GeniusAuthResponse> {
    fn from_env() -> Self {
        let client_id =
            env::var("GENIUS_CLIENT_ID").expect("Set the GENIUS_CLIENT_ID env variable ");
        let client_secret =
            env::var("GENIUS_CLIENT_SECRET").expect("Set the GENIUS_CLIENT_SECRET env variable ");
        Authorizer::<GeniusAuthResponse> {
            client_id,
            client_secret,
            scope: vec![],
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

#[cfg(test)]
mod tests {
    use crate::remove_feat;

    #[test]
    fn test_remove_feature_in_song_name() {
        let mut song_name = String::from("Skrawberries (feat. BJ The Chicago Kid)");
        let cleared_string = remove_feat(&mut song_name);
        let correct_string = String::from("Skrawberries");

        assert_eq!(correct_string, cleared_string);

        let mut song_name = String::from("CASH MANIAC | CAZH MAN1AC [FEAT. NYYJERYA]");
        let cleared_string = remove_feat(&mut song_name);
        let correct_string = String::from("CASH MANIAC | CAZH MAN1AC");

        assert_eq!(correct_string, cleared_string);

        let mut song_name = String::from("Beauty In The Dark (Groove With You)");
        let cleared_string = remove_feat(&mut song_name);
        let correct_string = String::from("Beauty In The Dark (Groove With You)");

        assert_eq!(correct_string, cleared_string);
    }
}
