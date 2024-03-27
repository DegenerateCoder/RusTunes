use crate::music_player::Error;
use crate::utils;

pub struct RemoteSourceProcessor {
    piped_api_domains: Vec<String>,
    piped_api_domain_index: usize,
    invidious_api_domains: Vec<String>,
    invidious_api_domain_index: usize,
    duration_limit: u64,
    piped_api_domain_index_start: usize,
    invidious_api_domain_index_start: usize,
}

#[derive(Debug, Clone)]
pub struct Remote {
    pub url: String,
    pub video_id: String,
    pub audio_stream_url: String,
    pub title: String,
    pub length: u64,
}

#[derive(Debug, Clone)]
pub struct Local {}

#[derive(Debug, Clone)]
pub enum Source {
    Remote(Remote),
    _Local(Local),
}

impl Source {
    pub fn new_remote(path: &str) -> Result<Self, Error> {
        Ok(Source::Remote(Remote::new(path)?))
    }

    pub fn get_remote_source(&self) -> Result<&Remote, Error> {
        match self {
            Source::Remote(remote_src) => Ok(remote_src),
            _ => Err(Error::OtherError("Not remote source".to_owned())),
        }
    }

    pub fn get_remote_source_mut(&mut self) -> Result<&mut Remote, Error> {
        match self {
            Source::Remote(remote_src) => Ok(remote_src),
            _ => Err(Error::OtherError("Not remote source".to_owned())),
        }
    }

    pub fn is_valid_source_path(path: &str) -> bool {
        let mut valid_path = false;
        valid_path |= Remote::url_into_video_id(path).is_ok();
        valid_path |= Remote::url_into_playlist_id(path).is_ok();

        valid_path
    }
}

impl Remote {
    pub fn new(path: &str) -> Result<Self, Error> {
        Ok(Remote {
            url: path.to_string(),
            video_id: Remote::url_into_video_id(path)?,
            audio_stream_url: "".to_string(),
            title: "".to_string(),
            length: 0,
        })
    }

    pub fn url_into_video_id(url: &str) -> Result<String, Error> {
        if url.contains("v=") {
            let split = url.split("v=");
            let id = split.last().unwrap().to_string();

            Ok(id)
        } else {
            Err(Error::InvalidVideoUrl(format!(
                "Not a valid youtube/piped video url: {url}, url must contain 'v='"
            )))
        }
    }

    pub fn url_into_playlist_id(url: &str) -> Result<String, Error> {
        if url.contains("list=") {
            let split = url.split("list=");
            let id = split.last().unwrap().to_string();

            Ok(id)
        } else {
            Err(Error::InvalidPlaylistUrl(format!(
                "Not a valid youtube/piped playlist url: {url}, url must contain 'list='"
            )))
        }
    }
}

impl RemoteSourceProcessor {
    pub fn new(
        piped_api_domains: Vec<String>,
        piped_api_domain_index: usize,
        invidious_api_domains: Vec<String>,
        invidious_api_domain_index: usize,
        duration_limit: u64,
    ) -> Result<Self, Error> {
        Ok(Self {
            piped_api_domains,
            piped_api_domain_index,
            invidious_api_domains,
            invidious_api_domain_index,
            duration_limit,
            piped_api_domain_index_start: piped_api_domain_index,
            invidious_api_domain_index_start: invidious_api_domain_index,
        })
    }

    pub fn next_piped_api_domains_index(&mut self) -> Result<(), Error> {
        let mut i = self.piped_api_domain_index;
        i += 1;
        if i >= self.piped_api_domains.len() {
            i = 0;
        }
        if i == self.piped_api_domain_index_start {
            Err(Error::AllPipedApiDomainsDown(
                "All piped api domains are unrechable".to_string(),
            ))
        } else {
            self.piped_api_domain_index = i;
            log::info!(
                "RemoteSourceProcessor::next_piped_api_domains_index -> {:?}",
                self.get_piped_api_domain()
            );
            Ok(())
        }
    }

    pub fn get_piped_api_domain(&self) -> &str {
        &self.piped_api_domains[self.piped_api_domain_index]
    }

    pub fn get_invidious_api_domain(&self) -> &str {
        &self.invidious_api_domains[self.invidious_api_domain_index]
    }

    pub fn set_audio_url_title(&mut self, source: &mut Remote) -> Result<(), Error> {
        let result = self._set_audio_url_title(source);

        if result.is_err() {
            log::info!("RemoteSourceProcessor::->set_audio_url_title {:?}", result);
        }

        Ok(result?)
    }

    pub fn _set_audio_url_title(&mut self, source: &mut Remote) -> Result<(), Error> {
        let request_url = format!(
            "{}/streams/{}",
            self.get_piped_api_domain(),
            &source.video_id
        );

        let mut response: serde_json::Value = utils::reqwest_get(&request_url)?.json()?;

        let _test_audio_streams = response
            .get("audioStreams")
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
        let audio_streams: &mut Vec<serde_json::Value> = response
            .get_mut("audioStreams")
            .unwrap()
            .as_array_mut()
            .unwrap();

        audio_streams.sort_by_key(|x| x.get("bitrate").unwrap().as_u64().unwrap());
        let audio_stream = audio_streams.last().unwrap();
        let music_url = audio_stream.get("url").unwrap();
        source.audio_stream_url = music_url.to_string();
        source.audio_stream_url = source.audio_stream_url.replace("\"", "");
        let music_title = response.get("title").unwrap();
        source.title = music_title.to_string();
        let duration = response.get("duration").unwrap();
        source.length = duration.as_u64().unwrap();

        self.piped_api_domain_index_start = self.piped_api_domain_index;
        Ok(())
    }

    pub fn get_video_genre(&mut self, source: &Remote) -> Result<String, Error> {
        let mut result = self._get_video_genre(source);

        while result.is_err() {
            log::info!("RemoteSourceProcessor::->get_video_genre {:?}", result);
            let update = self.next_invidious_api_domains_index();
            if update.is_err() {
                log::info!("RemoteSourceProcessor::->get_video_genre {:?}", update);
                return Err(update.unwrap_err());
            }
            result = self._get_video_genre(source);
        }

        Ok(result?)
    }

    pub fn _get_video_genre(&mut self, source: &Remote) -> Result<String, Error> {
        let request_url = format!(
            "{}/api/v1/videos/{}",
            self.get_invidious_api_domain(),
            &source.video_id
        );

        let response: serde_json::Value = utils::reqwest_get(&request_url)?.json()?;

        let genre = response
            .get("genre")
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
        let genre = genre.as_str().unwrap().to_string();

        self.invidious_api_domain_index_start = self.invidious_api_domain_index;
        Ok(genre)
    }

    pub fn get_related_video_source(
        &mut self,
        video_id: &str,
        played_video_ids: &Vec<String>,
    ) -> Result<Source, Error> {
        log::info!(
            "RemoteSourceProcessor::get_related_video_url -> {:?}",
            video_id
        );

        let result = self._get_related_video_source(video_id, played_video_ids);

        if result.is_err() {
            log::info!(
                "RemoteSourceProcessor::get_related_video_url -> Error: {:?}",
                result
            );
        }

        if result.is_ok() {
            log::info!(
                "RemoteSourceProcessor::get_related_video_url -> {:?} => {:?}",
                video_id,
                result
                    .as_ref()
                    .unwrap()
                    .get_remote_source()
                    .unwrap()
                    .video_id
            );
        }

        Ok(result?)
    }

    fn _get_related_video_source(
        &mut self,
        video_id: &str,
        played_video_ids: &Vec<String>,
    ) -> Result<Source, Error> {
        let request_url = format!("{}/streams/{}", self.get_piped_api_domain(), video_id);

        let response: serde_json::Value = utils::reqwest_get(&request_url)
            .map_err(|err| match err {
                Error::ReqwestError(_) => Error::NoRelatedVideoFound(format!(
                    "No related videos found for video_id: {}",
                    video_id
                )),
                _ => err,
            })?
            .json()?;

        let related_streams = response
            .get("relatedStreams")
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
        let related_streams: &Vec<serde_json::Value> = related_streams.as_array().unwrap();

        for related_stream in related_streams {
            let related_video_url = related_stream.get("url").unwrap();
            let related_video_url = related_video_url.as_str().unwrap();
            if related_video_url.contains("/playlist") {
                continue;
            }
            if self.check_filters_for_related_video_url(
                related_video_url,
                related_stream,
                played_video_ids,
            )? {
                //println!("Next to play: {related_video_url} <- from {video_id}");

                self.piped_api_domain_index_start = self.piped_api_domain_index;
                return Ok(Source::new_remote(related_video_url)?);
            }
        }
        Err(Error::NoRelatedVideoFound(format!(
            "No related videos found for video_id: {}",
            video_id
        )))
    }

    fn check_filters_for_related_video_url(
        &mut self,
        video_url: &str,
        stream_json: &serde_json::Value,
        played_video_ids: &Vec<String>,
    ) -> Result<bool, Error> {
        let new_remote_src = Source::new_remote(video_url)?;
        match new_remote_src {
            Source::Remote(remote_src) => {
                let video_id = &remote_src.video_id;
                if played_video_ids.contains(video_id) {
                    return Ok(false);
                } else if video_id.contains("list=") {
                    //MIX or playlist
                    return Ok(false);
                } else if stream_json // possiblty of MIX with no duration
                    .get("duration")
                    .ok_or_else(|| Error::OtherError(format!("{:?}", stream_json)))?
                    .as_u64()
                    .unwrap_or(self.duration_limit + 1)
                    > self.duration_limit
                {
                    return Ok(false);
                } else if !self.get_video_genre(&remote_src)?.contains("Music") {
                    return Ok(false);
                }
            }
            _ => panic!(),
        }
        Ok(true)
    }

    pub fn playlist_to_remote_vec(&mut self, playlist_id: &str) -> Result<Vec<Source>, Error> {
        let result = self._playlist_to_remote_vec(playlist_id);

        if result.is_err() {
            log::info!(
                "RemoteSourceProcessor::playlist_to_remote_vec -> {:?}",
                result
            );
        }

        Ok(result?)
    }

    pub fn _playlist_to_remote_vec(&mut self, playlist_id: &str) -> Result<Vec<Source>, Error> {
        let mut playlist = Vec::new();
        let request_url = format!("{}/playlists/{}", self.get_piped_api_domain(), playlist_id);

        let mut response: serde_json::Value = utils::reqwest_get(&request_url)?.json()?;

        loop {
            let related_streams = response
                .get("relatedStreams")
                .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
            let related_streams: &Vec<serde_json::Value> = related_streams.as_array().unwrap();

            for stream in related_streams {
                let url = stream.get("url").unwrap().as_str().unwrap().to_string();
                let video_id = Remote::url_into_video_id(&url).unwrap();

                playlist.push(Source::Remote(Remote {
                    url,
                    video_id,
                    audio_stream_url: "".to_string(),
                    title: "".to_string(),
                    length: 0,
                }));
            }

            let nextpage = response.get("nextpage").unwrap();
            if nextpage.is_null() {
                break;
            }

            let nextpage = nextpage.as_str().unwrap().to_owned();
            let request_url = format!(
                "{}/nextpage/playlists/{}?nextpage={}",
                self.get_piped_api_domain(),
                playlist_id,
                urlencoding::encode(&nextpage)
            );

            response = utils::reqwest_get(&request_url)?.json()?;
        }

        self.piped_api_domain_index_start = self.piped_api_domain_index;
        Ok(playlist)
    }

    pub fn fetch_piped_api_domains(&mut self) -> Result<(), Error> {
        log::info!("RemoteSourceProcessor::fetch_piped_api_domains");

        self.piped_api_domains.clear();
        self.piped_api_domain_index = 0;
        self.piped_api_domain_index_start = 0;

        let result = utils::fetch_piped_api_domains();
        if result.is_err() {
            log::info!(
                "RemoteSourceProcessor::fetch_piped_api_domains -> {:?}",
                result
            );
        }

        self.piped_api_domains = result?;

        log::info!(
            "RemoteSourceProcessor::fetch_piped_api_domains -> {:?}",
            self.get_piped_api_domain()
        );

        Ok(())
    }

    pub fn fetch_invidious_api_domains(&mut self) -> Result<(), Error> {
        log::info!("RemoteSourceProcessor::fetch_invidious_api_domains");

        self.invidious_api_domains.clear();
        self.invidious_api_domain_index = 0;
        self.invidious_api_domain_index_start = 0;

        let result = utils::fetch_invidious_api_domains();
        if result.is_err() {
            log::info!(
                "RemoteSourceProcessor::fetch_invidious_api_domains -> {:?}",
                result
            );
        }

        self.invidious_api_domains = result?;

        log::info!(
            "RemoteSourceProcessor::fetch_invidious_api_domains -> {:?}",
            self.get_invidious_api_domain()
        );

        Ok(())
    }

    pub fn next_invidious_api_domains_index(&mut self) -> Result<(), Error> {
        let mut i = self.invidious_api_domain_index;
        i += 1;
        if i >= self.invidious_api_domains.len() {
            i = 0;
        }
        if i == self.invidious_api_domain_index_start {
            Err(Error::AllInvidiousApiDomainsDown(
                "All invidious api domains are unrechable".to_string(),
            ))
        } else {
            self.invidious_api_domain_index = i;
            log::info!(
                "RemoteSourceProcessor::next_invidious_api_domains_index -> {:?}",
                self.get_invidious_api_domain()
            );
            Ok(())
        }
    }

    pub fn is_valid_video_url(&mut self, url: &str) -> Result<bool, Error> {
        log::info!("RemoteSourceProcessor::is_valid_video_url -> {}", url);
        let result = self._is_valid_video_url(url);
        log::info!("RemoteSourceProcessor::is_valid_video_url -> {:?}", result);

        Ok(result?)
    }

    pub fn _is_valid_video_url(&self, url: &str) -> Result<bool, Error> {
        let video_id = Remote::url_into_video_id(url)?;
        let request_url = format!("{}/streams/{}", self.get_piped_api_domain(), video_id);
        let response = utils::reqwest_get(&request_url);

        if Self::invalid_data_status(&response) {
            Ok(false)
        } else {
            Ok(response?.status() == reqwest::StatusCode::OK)
        }
    }

    pub fn is_valid_playlist_url(&mut self, url: &str) -> Result<bool, Error> {
        log::info!("RemoteSourceProcessor::is_valid_playlist_url -> {}", url);
        let result = self._is_valid_playlist_url(url);
        log::info!(
            "RemoteSourceProcessor::is_valid_playlist_url -> {:?}",
            result
        );

        Ok(result?)
    }

    pub fn _is_valid_playlist_url(&self, url: &str) -> Result<bool, Error> {
        let playlist_id = Remote::url_into_playlist_id(url)?;
        let request_url = format!("{}/playlists/{}", self.get_piped_api_domain(), playlist_id);
        let response = utils::reqwest_get(&request_url);

        if Self::invalid_data_status(&response) {
            Ok(false)
        } else {
            Ok(response?.status() == reqwest::StatusCode::OK)
        }
    }

    fn invalid_data_status(response: &Result<reqwest::blocking::Response, Error>) -> bool {
        let mut invalid = false;
        if response.is_err() {
            let response_err = response.as_ref().unwrap_err();
            match response_err {
                Error::ReqwestError(response_err) => {
                    let response_status = response_err.status();
                    match response_status {
                        Some(reqwest::StatusCode::BAD_REQUEST) => invalid = true,
                        Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR) => invalid = true,
                        _ => (),
                    }
                }
                Error::VideoBlockedInAllRegions => {
                    invalid = true;
                }

                Error::VideoBlockedOnCopyRightGrounds => {
                    invalid = true;
                }
                _ => (),
            }
        }

        invalid
    }
}
