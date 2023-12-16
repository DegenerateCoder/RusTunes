#[derive(Debug)]
pub enum Error {
    InvalidVideoUrl(String),
    InvalidPlaylistUrl(String),
    ReqwestError(reqwest::Error),
    VideoBlockedInAllRegions,
    NoRelatedVideoFound(String),
    AllPipedApiDomainsDown(String),
    AllInvidiousApiDomainsDown(String),
    StdIOError(std::io::Error),
    OtherError(String),
    SerdeJSONError(serde_json::Error),
    PrintHelp,
    InvalidOption(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ReqwestError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::StdIOError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeJSONError(err)
    }
}
