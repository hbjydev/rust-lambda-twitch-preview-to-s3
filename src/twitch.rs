use serde::{Serialize, Deserialize};
use reqwest::{Client, Error};

#[derive(Serialize, Deserialize, Debug)]
pub struct TwitchStream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,

    #[serde(rename = "type")]
    pub tw_type: String,

    pub title: String,
    pub tags: Vec<String>,
    pub viewer_count: u16,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
    pub tag_ids: Vec<String>,
    pub is_mature: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TwitchPagination {
    pub cursor: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TwitchStreams {
    pub data: Vec<TwitchStream>,
    // pagination: TwitchPagination,
}

pub struct TwitchConfig {
    pub client_id: String,
    pub client_secret: String,
}

pub struct TwitchClient {
    config: TwitchConfig,
}

#[derive(Deserialize)]
struct TwitchIdOauth2Token {
    access_token: String,
    token_type: String,
    expires_in: u16,
}

impl TwitchClient {
    pub fn new(config: TwitchConfig) -> Self {
        Self { config }
    }

    async fn get_auth_token(&self) -> Result<TwitchIdOauth2Token, Error> {
        let client = Client::new();

        let token = client.post("https://id.twitch.tv/oauth2/token")
            .body(
                format!(
                    "client_id={}&client_secret={}&grant_type=client_credentials",
                    self.config.client_id,
                    self.config.client_secret,
                )
            )
            .header("Content-Type", "x-www-form-urlencoded");

        let token_res = token.send().await?.json::<TwitchIdOauth2Token>().await?;

        Ok(token_res)
    }

    pub async fn get_streams(&self, login: &str) -> Result<TwitchStreams, Error> {
        let token = self.get_auth_token().await?;
        let client = reqwest::Client::new();

        let res = client.get("https://api.twitch.tv/helix/streams")
            .query(&[("user_login", login)])
            .header("Client-Id", self.config.client_id.clone())
            .header("Authentication", format!("Bearer {}", token.access_token))
            .send()
            .await?;

        Ok(res.json::<TwitchStreams>().await?)
    }
}
