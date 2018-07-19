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
#[macro_use]
extern crate failure;

use app_dirs::{app_dir, get_app_dir, AppDataType, AppInfo};
use clap::{App, Arg};
use failure::Error;
use hyper::header::{Authorization, Bearer, Headers};
use std::fs::File;
use std::io::prelude::*;

const APP_INFO: AppInfo = AppInfo {
    name: crate_name!(),
    author: crate_authors!(),
};

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

#[derive(Debug)]
struct Options<'a> {
    api_key: &'a str,
    api_url: &'a str,
    org: &'a str,
    clear_cache: bool,
}

#[derive(Fail, Debug)]
#[fail(display = "Project not found")]
pub struct ProjectNotFoundError;

#[derive(Fail, Debug)]
#[fail(display = "Sentry API Error: {}", body)]
pub struct SentryApiError {
    body: String,
}

fn main() {
    let matches = App::new("Sentry project lookup")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about("Look up sentry project slugs by id using the api")
        .arg(
            Arg::with_name("api-key")
                .long("api-key")
                .env("SENTRY_APIKEY")
                .help("Sentry API key")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("api-url")
                .long("api-url")
                .env("SENTRY_URL")
                .help("Sentry API url")
                .takes_value(true)
                .default_value("https://sentry.io"),
        )
        .arg(
            Arg::with_name("org")
                .long("org")
                .env("SENTRY_ORG")
                .help("Sentry organisation")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("project-id")
                .index(1)
                .help("The project id to look up")
                .takes_value(true)
                .required(true),
        )
        .arg_from_usage("--clear-cache 'clear the project cache'")
        .get_matches();

    let project_id = matches.value_of("project-id").unwrap();
    let options = Options {
        api_key: matches.value_of("api-key").unwrap(),
        api_url: matches.value_of("api-url").unwrap(),
        org: matches.value_of("org").unwrap(),
        clear_cache: matches.is_present("clear-cache"),
    };
    let slug = get_slug(project_id, &options).unwrap_or_else(|err| {
        println!("{}", err);
        ::std::process::exit(1);
    });
    println!("{}", slug);
}

fn get_slug(project_id: &str, options: &Options) -> Result<String, Error> {
    let projects = get_projects(options)?;

    for project in &projects {
        if project.id == project_id {
            return Ok(project.slug.clone());
        }
    }

    Err(ProjectNotFoundError)?
}

fn get_projects(options: &Options) -> Result<Vec<Project>, Error> {
    if !options.clear_cache {
        if let Ok(projects) = get_cache() {
            return Ok(projects);
        }
    }
    let projects = get_projects_from_api(options)?;
    set_cache(&projects)?;
    Ok(projects)
}

fn get_projects_from_api(options: &Options) -> Result<Vec<Project>, Error> {
    let mut headers = Headers::new();
    headers.set(Authorization(Bearer {
        token: options.api_key.into(),
    }));
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/0/organizations/{}/projects/",
        options.api_url, options.org,
    );
    let mut res = client.get(&url).headers(headers).send()?;

    if !res.status().is_success() {
        let body = res.text()?;
        return Err(SentryApiError { body })?;
    }

    let projects: Vec<Project> = res.json()?;
    Ok(projects)
}

fn set_cache(projects: &[Project]) -> Result<(), Error> {
    let contents = json!(projects);

    let cache_dir = app_dir(AppDataType::UserCache, &APP_INFO, "cache")?;
    let filename = cache_dir.join(PROJECT_CACHE_FILENAME);
    let mut file = File::create(filename)?;

    write!(file, "{}", contents)?;
    Ok(())
}

fn get_cache() -> Result<Vec<Project>, Error> {
    let cache_dir = get_app_dir(AppDataType::UserCache, &APP_INFO, "cache")?;
    let filename = cache_dir.join(PROJECT_CACHE_FILENAME);
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let projects = serde_json::from_str(&contents)?;

    Ok(projects)
}
