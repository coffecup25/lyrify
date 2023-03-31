mod genius_auth;
mod spotify_auth;

use crate::response::AccessTokenQuery;
use serde::Deserialize;
use std::marker::PhantomData;

/// A generic struct that is used to authorize against API:s.
/// It includes the client_id, client_secret, scope, redirect_uri, state, and
/// endpoints.
pub struct Authorizer<T> {
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
    T: AccessTokenQuery + for<'de> Deserialize<'de>,
{
    /// Authorizes the application using the Authorization Code Flow for authorizer of type ```T``` 
    /// 
    /// The workflow is as follows:
    /// 1. The user is prompted to authorize the application at spotify's website.
    /// 2. The user is redirected to the redirect_uri with a query parameter containing the authorization code.
    /// 3. The authorization code is exchanged for an access token and a refresh token.
    /// 4. The access token is used to query the API.
    /// 5. If the access token expires, the refresh token is used to get a new access token.
    /// 
    fn authorize(&self) -> T {
        let client = reqwest::blocking::Client::new();

        // get the url for the authorization page
        let auth_url = self.auth_url(&client);

        // open the url in the default browser so the user can sign in and authorize the application
        open::that(auth_url).unwrap(); 

        println!("Paste the url from the browser"); // todo: start server that listens on a local adress instead

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();

        // the authorization code is in the query parameter of the url
        let response_url = reqwest::Url::parse(&buf).unwrap();
        let mut queries = response_url.query_pairs();

        let (key, val) = queries.next().unwrap();
        let val = match queries.next().unwrap() {
            (key,val) if key == "code" => val,
            (key,val) if key == "error" => panic!("{}", val), // todo: return error instead
            _ => panic!("response_url isnt correct")
        };
        let auth_code = val;

        self.exchange_auth_code(&client, &auth_code)
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
    /// Exchanges the authorization code for an access token and a refresh token.
    /// 
    /// On success the following fields are returned
    /// 
    /// | KEY           | VALUE TYPE | VALUE DESCRIPTION |                                                                                                                                                                               
    /// |---------------|------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
    /// | access_token  | string     | An Access Token that can be provided in subsequent calls, for example to Spotify Web API services.                                                                                              |
    /// | token_type    | string     | How the Access Token may be used: always "Bearer".                                                                                                                                               |
    /// | scope         | string     | A space-separated list of scopes which have been granted for this access_token                                                                                                                  |
    /// | expires_in    | int        | The time period (in seconds) for which the Access Token is valid.                                                                                                                                |
    /// | refresh_token | string     | A token that can be sent to the Spotify Accounts service in place of an authorization code. (When the access code expires, send a POST request to the Accounts service /api/token endpoint, but use this code in place of an authorization code. A new Access Token will be returned. A new refresh token might be returned too.) |

    fn exchange_auth_code(&self, client: &reqwest::blocking::Client, auth_code: &str) -> T {
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
