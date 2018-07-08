extern crate app_dirs;
#[macro_use]
extern crate clap;
extern crate hyper;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use app_dirs::{AppInfo, AppDataType, app_dir, get_app_dir};
use clap::{App, Arg, SubCommand};
use hyper::header::{Authorization, Bearer, Headers};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;


const APP_INFO: AppInfo = AppInfo{name: "sentry-api", author: "David Szotten"};

const SENTRY_ORG: &str = "org";
const PROJECT_CACHE_FILENAME: &str = "projects.json";

#[derive(Debug, Deserialize, Serialize)]
struct Response {
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Project {
    id: String,
    slug: String,
}

fn main() -> Result<(), Box<Error>> {
    let matches = App::new("Sentry api client")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about("Look up stuff using the sentry api")
        .subcommand(SubCommand::with_name("get-slug")
            .about("Find a project slug by id")
            .arg(
                Arg::with_name("project_id")
                    .index(1)
                    .value_name("PROJECT-ID")
                    .help("The project id to look up")
                    .takes_value(true)
                    .required(true),
            )
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("get-slug") {
        let project_id = matches.value_of("project_id").unwrap();
        let slug = get_slug(project_id)?;
        println!("{}", slug);
    }

    // get_cache()?;
    return Ok(());

}


fn get_slug(project_id: &str) -> Result<String, Box<Error>> {
    let projects = get_projects()?;

    for project in projects.iter() {
        if project.id == project_id {
            return Ok(project.slug.clone());
        }
    }

    Err("Project not found")?
}


fn get_projects() -> Result<Vec<Project>, Box<Error>> {
    if let Ok(projects) = get_cache() {
        return Ok(projects);
    }

    let api_key = match env::var("SENTRY_APIKEY") {
        Ok(val) => Ok(val),
        Err(_) => Err("SENTRY_APIKEY missing"),
    }?;

    let mut headers = Headers::new();
    headers.set(Authorization(Bearer { token: api_key }));
    let client = reqwest::Client::new();
    let url = format!(
        "https://sentry.io/api/0/organizations/{}/projects/",
        SENTRY_ORG
    );
    let mut res = client
        .get(&url)
        .headers(headers)
        .send()?;

    if !res.status().is_success() {
        let body = res.text()?;
        return Err(body.into());
    }

    let projects: Vec<Project> = res.json()?;

    set_cache(&projects)?;
    Ok(projects)
}

fn set_cache(projects: &[Project]) -> Result<(), Box<Error>> {
    let contents = json!(projects);

    let cache_dir = app_dir(AppDataType::UserCache, &APP_INFO, "cache")?;
    let filename = cache_dir.join(PROJECT_CACHE_FILENAME);
    let mut file = File::create(filename)?;

    write!(file, "{}", contents)?;
    Ok(())
}


fn get_cache() -> Result<Vec<Project>, Box<Error>> {
    let cache_dir = get_app_dir(AppDataType::UserCache, &APP_INFO, "cache")?;
    let filename = cache_dir.join(PROJECT_CACHE_FILENAME);
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let projects = serde_json::from_str(&contents)?;

    Ok(projects)
}
