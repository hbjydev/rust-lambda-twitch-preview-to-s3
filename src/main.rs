use aws_config::load_from_env;
use aws_sdk_s3::Client; 
use aws_lambda_events::event::cloudwatch_events::CloudWatchEvent;
use aws_sdk_s3::types::ByteStream;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Serialize,Deserialize};

mod twitch;

#[derive(Serialize, Deserialize)]
struct TwitchEventSubStreamOnline {
    twitch_user_login: String,
}

struct FuncEnv {
    bucket_name: String,

    twitch_client_id: String,
    twitch_client_secret: String,
}

async fn get_twitch_client(env: &FuncEnv) -> twitch::TwitchClient {
    twitch::TwitchClient::new(twitch::TwitchConfig {
        client_id: env.twitch_client_id.clone(),
        client_secret: env.twitch_client_secret.clone(),
    })
}

async fn get_s3_client() -> Client {
    let config = load_from_env().await;
    let client = Client::new(&config);

    return client;
} 

fn get_env() -> Result<FuncEnv, std::env::VarError> {
    let bucket_name = std::env::var("BUCKET_NAME")?;
    let twitch_client_secret = std::env::var("TWITCH_CLIENT_SECRET")?;
    let twitch_client_id = std::env::var("TWITCH_CLIENT_ID")?;

    Ok(FuncEnv {
        bucket_name,
        twitch_client_secret,
        twitch_client_id,
    })
}

async fn function_handler(event: LambdaEvent<CloudWatchEvent<TwitchEventSubStreamOnline>>) -> Result<(), Error> {
    let client = get_s3_client().await;
    let env = get_env()?;
    let tw_client = get_twitch_client(&env).await;
    let detail = event.payload.detail.as_ref().unwrap();

    let streams = tw_client.get_streams(&detail.twitch_user_login).await?;

    if streams.data.len() == 0 {
        return Err("Stream list did not include a live stream.".into());
    }

    let thumbnail_url = &streams.data.get(0).unwrap().thumbnail_url;
    let thumbnail_set = thumbnail_url.replace("{width}", "1280").replace("{height}", "720");
    let thumbnail = reqwest::get(thumbnail_set).await?.bytes().await?;

    let put = client.put_object()
        .key(format!("{}.jpg", detail.clone().twitch_user_login))
        .bucket(env.bucket_name.clone())
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
