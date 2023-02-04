use aws_config::load_from_env;
use aws_sdk_s3::Client; 
use aws_lambda_events::event::cloudwatch_events::CloudWatchEvent;
use aws_sdk_s3::types::ByteStream;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize)]
struct TwitchEventSubStreamOnline {
    twitch_user_login: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TwitchStream {
    id: String,
    user_id: String,
    user_login: String,
    user_name: String,
    game_id: String,
    game_name: String,

    #[serde(rename = "type")]
    tw_type: String,

    title: String,
    tags: Vec<String>,
    viewer_count: u16,
    started_at: String,
    language: String,
    thumbnail_url: String,
    tag_ids: Vec<String>,
    is_mature: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct TwitchPagination {
    cursor: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TwitchStreams {
    data: Vec<TwitchStream>,
    // pagination: TwitchPagination,
}

struct FuncEnv {
    bucket_name: String,

    twitch_oauth2_token: String,
    twitch_client_id: String,
}

fn get_twitch_client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn get_s3_client() -> Client {
    let config = load_from_env().await;
    let client = Client::new(&config);

    return client;
} 

fn get_env() -> FuncEnv {
    let tot = match std::env::var("TWITCH_OAUTH2_TOKEN") {
        Ok(val) => val,
        Err(_e) => String::from("none"),
    };

    let tci = match std::env::var("TWITCH_CLIENT_ID") {
        Ok(val) => val,
        Err(_e) => String::from("none"),
    };

    let bn = match std::env::var("BUCKET_NAME") {
        Ok(val) => val,
        Err(_e) => String::from(""),
    };

    FuncEnv {
        bucket_name: bn,

        twitch_oauth2_token: tot,
        twitch_client_id: tci,
    }
}

async fn function_handler(event: LambdaEvent<CloudWatchEvent<TwitchEventSubStreamOnline>>) -> Result<(), Error> {
    let client = get_s3_client().await;
    let tw_client = get_twitch_client();
    let detail = event.payload.detail.as_ref().unwrap();
    let env = get_env();

    let streams = tw_client
        .get("https://api.twitch.tv/helix/streams")
        .query(&[
            ("user_login", detail.twitch_user_login.clone()),
            ("type", "live".to_string()),
        ])
        .header("Authorization", format!("Bearer {}", env.twitch_oauth2_token))
        .header("Client-Id", env.twitch_client_id)
        .send()
        .await?
        .json::<TwitchStreams>()
        .await?;

    if streams.data.len() == 0 {
        return Err("Stream list did not include a live stream.".into());
    }

    let thumbnail_url = &streams.data.get(0).unwrap().thumbnail_url;
    let thumbnail_set = thumbnail_url.replace("{width}", "1280").replace("{height}", "720");
    let thumbnail = tw_client.get(thumbnail_set).send().await?.bytes().await?;

    let put = client.put_object()
        .key(format!("{}.jpg", detail.clone().twitch_user_login))
        .bucket(env.bucket_name)
        .body(ByteStream::from(thumbnail))
        .send()
        .await;

    match put {
        Ok(_v) => Ok(()),
        Err(e) => {
            println!("{:?}", e);

            Err(Into::into("Failed to upload thumbnail to S3."))
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
