use crate::music_player::error::Error;

#[inline(always)]
pub fn get_reqwest_client() -> reqwest::Result<reqwest::blocking::Client> {
    let user_agent: String = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let reqwest_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent(user_agent)
        .build()?;

    Ok(reqwest_client)
}

#[inline(always)]
pub fn reqwest_get(url: &str) -> reqwest::Result<reqwest::blocking::Response> {
    let reqwest_client = get_reqwest_client()?;

    let request = reqwest_client.get(url).build()?;
    let response = reqwest_client.execute(request)?;

    Ok(response.error_for_status()?)
}

#[inline(always)]
pub fn measure_reqwest_get_duration(url: &str) -> reqwest::Result<std::time::Duration> {
    let reqwest_client = get_reqwest_client()?;

    let request = reqwest_client.get(url).build()?;

    let start = std::time::SystemTime::now();
    let response = reqwest_client.execute(request)?;
    let elapsed = start.elapsed().unwrap();
    response.error_for_status()?;

    Ok(elapsed)
}

pub fn fetch_piped_api_domains() -> Result<Vec<String>, Error> {
    let mut piped_api_domains = Vec::new();

    let request_url = "https://piped-instances.kavin.rocks/";
    let response: serde_json::Value = reqwest_get(&request_url)?.json()?;

    let instances = response
        .as_array()
        .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
    for instance in instances {
        let api_url = instance
            .get("api_url")
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
        let api_url = api_url.as_str().unwrap();

        piped_api_domains.push(api_url.to_string());
    }

    Ok(piped_api_domains)
}

pub fn fetch_invidious_api_domains() -> Result<Vec<String>, Error> {
    let mut invidious_api_domains = Vec::new();

    let request_url = "https://api.invidious.io/instances.json?pretty=0&sort_by=type,health";
    let response: serde_json::Value = reqwest_get(&request_url)?.json()?;

    let instances = response
        .as_array()
        .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;

    for instance in instances {
        let instance_data = instance
            .get(1)
            .ok_or_else(|| Error::OtherError(format!("{:?}", instance.to_string())))?;
        let api = instance_data
            .get("api")
            .ok_or_else(|| Error::OtherError(format!("{:?}", instance.to_string())))?
            .as_bool();
        if let Some(api) = api {
            if !api {
                continue;
            }
        } else {
            continue;
        }
        let api_url = instance_data
            .get("uri")
            .ok_or_else(|| Error::OtherError(format!("{:?}", instance.to_string())))?;
        let api_url = api_url.as_str().unwrap();

        invidious_api_domains.push(api_url.to_string());
    }

    Ok(invidious_api_domains)
}
