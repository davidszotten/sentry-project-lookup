#[macro_use]
extern crate clap;
extern crate hyper;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use clap::{App, Arg, SubCommand};
use hyper::header::{Authorization, Bearer, Headers};
use std::env;
use std::error::Error;

const SENTRY_ORG: &str = "org";

#[derive(Debug, Deserialize)]
struct Response {
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize)]
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
    return Ok(());

}


fn get_slug(project_id: &str) -> Result<String, Box<Error>> {
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
    // println!("{:?}", projects);

    for project in projects.iter() {
        if project.id == project_id {
            return Ok(project.slug.clone());
            // println!("{}", project.slug);
        }
    }

    Err("Project not found")?
}
