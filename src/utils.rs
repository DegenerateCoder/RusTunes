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
