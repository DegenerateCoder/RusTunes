#[inline(always)]
pub fn get_reqwest_client() -> reqwest::Result<reqwest::blocking::Client> {
    let reqwest_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
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
