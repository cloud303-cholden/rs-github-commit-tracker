use std::collections::HashMap;
use std::env;

use aws_lambda_events::{
    apigw::ApiGatewayProxyResponse as Response,
    encodings::Body,
};
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use http::header::{HeaderMap, CONTENT_TYPE};
use lambda_runtime::{
    run,
    service_fn,
    Error,
    LambdaEvent,
};
use octocrab::models;
use serde::Deserialize;
use svg::Document;
use svg::Node;
use svg::node;
use svg::node::element::Style;
use svg::node::element::{Rectangle, Title, Text};
use tracing::log;

#[derive(Deserialize)]
struct Request {}

async fn function_handler(_event: LambdaEvent<Request>) -> Result<Response, Box<dyn std::error::Error>> {
    log::set_max_level(log::LevelFilter::Warn);

    let allowed_filetypes = [
        "rs".to_string(),
        "py".to_string(),
        "lua".to_string(),
        "go".to_string(),
        "ts".to_string(),
        "vue".to_string(),
    ];

    let github = octocrab::OctocrabBuilder::default()
        .user_access_token(env::var("GH_TOKEN")?)
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "image/svg+xml; charset=utf-8".parse().unwrap());

    let mut document = Document::new()
        .set("viewBox", (0, 0, 500, 700))
        .set("width", 500)
        .set("height", 700)
        .set("xmlns", "http://www.w3.org/2000/svg")
        .add(Title::new()
            .add(node::Text::new("Commit Tracker"))
        )
        .add(Style::new(
            ".description { font: 400 13px 'Segoe UI', Ubuntu, Sans-Serif; fill: #d8dee9 }"
        ))
        .add(Rectangle::new()
            .set("x", 0.5)
            .set("y", 0.5)
            .set("width", 499)
            .set("height", "99%")
            .set("fill", "#2e3440")
            .set("stroke", "#e4e2e2")
            .set("rx", 4.5)
            .set("stroke-opacity", 0)
        );

    let mut stats = HashMap::<String, u64>::new();
    let gh_config = env::var("GH_CONFIG")?;
    let config: HashMap<String, Vec<String>> = serde_json::from_str(&gh_config)?;
    let since: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    for (owner, repos) in config {
        for repo in repos {
            println!("{}", &repo);

            let mut page = github
                .repos(&owner, &repo)
                .list_commits()
                .author("cloud303-cholden")
                .since(since)
                .per_page(100)
                .send()
                .await?;

            loop {
                for commit in &page {
                    let resp = github
                        .commits(&owner, &repo)
                        .get(commit.sha.to_string())
                        .await?;

                    for file in resp.files.unwrap() {
                        let filetype = file
                            .filename
                            .split('.')
                            .last()
                            .unwrap()
                            .to_string();
                        if !allowed_filetypes.contains(&filetype) {
                            continue
                        };
                        *stats
                            .entry(filetype)
                            .or_insert(file.changes) += file.changes;

                    }
                }
                page = match github
                    .get_page::<models::repos::RepoCommit>(&page.next)
                    .await?
                {
                    Some(next_page) => next_page,
                    None => break,
                }
            }
        }
    }

    for (i, (filetype, changes)) in stats.into_iter().enumerate() {
        let desc = format!("{filetype}: {changes}");
        document.append(Text::new()
            .set("class", "description")
            .set("x", 25)
            .set("y", 18 * (i + 1))
            .add(node::Text::new(desc))
        );
    }

    let resp = Response {
        status_code: 200,
        body: Some(Body::Text(document.to_string())),
        headers,
        multi_value_headers: HeaderMap::new(),
        is_base64_encoded: None,
    };
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let env_filter = env::var("ENV_FILTER")
        .unwrap_or("aws_config=warn,hyper=info".to_string());
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    run(service_fn(function_handler)).await
}
