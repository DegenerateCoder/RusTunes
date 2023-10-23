use crate::music_player::logger::{Error, LogSender};

pub struct RemoteSourceProcessor {
    piped_api_domains: Vec<String>,
    piped_api_domain_index: usize,
    invidious_api_domains: Vec<String>,
    invidious_api_domain_index: usize,
    duration_limit: u64,
    piped_api_domain_index_start: usize,
    log_send: LogSender,
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
        log_send: crate::music_player::logger::LogSender,
    ) -> Self {
        Self {
            piped_api_domains,
            piped_api_domain_index,
            invidious_api_domains,
            invidious_api_domain_index,
            duration_limit,
            piped_api_domain_index_start: piped_api_domain_index,
            log_send,
        }
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
            self.log_send.send_log_message(format!(
                "RemoteSourceProcessor::next_piped_api_domains_index -> {:?}",
                self.get_piped_api_domain()
            ));
            self.piped_api_domain_index = i;
            Ok(())
        }
    }

    pub fn get_piped_api_domain(&self) -> &str {
        &self.piped_api_domains[self.piped_api_domain_index]
    }

    pub fn get_invidious_api_domains(&self) -> &str {
        &self.invidious_api_domains[self.invidious_api_domain_index]
    }

    pub fn set_audio_url_title(&mut self, source: &mut Remote) -> Result<(), Error> {
        let result = self._set_audio_url_title(source);

        if result.is_err() {
            self.log_send.send_log_message(format!(
                "RemoteSourceProcessor::->set_audio_url_title {:?}",
                result
            ));
        }

        Ok(result?)
    }

    pub fn _set_audio_url_title(&mut self, source: &mut Remote) -> Result<(), Error> {
        let request_url = format!(
            "{}/streams/{}",
            self.get_piped_api_domain(),
            &source.video_id
        );
        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)?.json()?;

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

    pub fn get_video_genre(&self, source: &Remote) -> Result<String, Error> {
        let result = self._get_video_genre(source);

        if result.is_err() {
            self.log_send.send_log_message(format!(
                "RemoteSourceProcessor::->get_video_genre {:?}",
                result
            ));
        }

        Ok(result?)
    }

    pub fn _get_video_genre(&self, source: &Remote) -> Result<String, Error> {
        let request_url = format!(
            "{}/api/v1/videos/{}",
            self.get_invidious_api_domains(),
            &source.video_id
        );
        let response: serde_json::Value = reqwest::blocking::get(&request_url)?.json()?;

        let genre = response
            .get("genre")
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
        let genre = genre.as_str().unwrap().to_string();

        Ok(genre)
    }

    pub fn get_related_video_url(
        &mut self,
        video_id: &str,
        played_video_ids: &Vec<String>,
    ) -> Result<Source, Error> {
        let result = self._get_related_video_url(video_id, played_video_ids);

        if result.is_err() {
            self.log_send.send_log_message(format!(
                "RemoteSourceProcessor::get_related_video_url -> {:?}",
                result
            ));
        }

        Ok(result?)
    }

    fn _get_related_video_url(
        &mut self,
        video_id: &str,
        played_video_ids: &Vec<String>,
    ) -> Result<Source, Error> {
        let request_url = format!("{}/streams/{}", self.get_piped_api_domain(), video_id);
        let response: serde_json::Value = reqwest::blocking::get(&request_url)?.json()?;
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
        &self,
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
                } else if stream_json.get("duration").unwrap().as_u64().unwrap_or(self.duration_limit+1)// .unwrap()
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
            self.log_send.send_log_message(format!(
                "RemoteSourceProcessor::playlist_to_remote_vec -> {:?}",
                result
            ));
        }

        Ok(result?)
    }

    pub fn _playlist_to_remote_vec(&mut self, playlist_id: &str) -> Result<Vec<Source>, Error> {
        let mut playlist = Vec::new();
        let request_url = format!("{}/playlists/{}", self.get_piped_api_domain(), playlist_id);

        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)?.json()?;

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

            response = reqwest::blocking::get(&request_url)?.json()?;
        }

        self.piped_api_domain_index_start = self.piped_api_domain_index;
        Ok(playlist)
    }

    pub fn fetch_piped_api_domains(&mut self) -> Result<(), Error> {
        let result = self._fetch_piped_api_domains();

        if result.is_err() {
            self.log_send.send_log_message(format!(
                "RemoteSourceProcessor::playlist_id -> {:?}",
                result
            ));
        }

        Ok(result?)
    }

    pub fn _fetch_piped_api_domains(&mut self) -> Result<(), Error> {
        let request_url = "https://piped-instances.kavin.rocks/";

        let response: serde_json::Value = reqwest::blocking::get(request_url)?.json()?;
        let instances = response
            .as_array()
            .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;

        self.piped_api_domains.clear();

        for instance in instances {
            let api_url = instance
                .get("api_url")
                .ok_or_else(|| Error::OtherError(format!("{:?}", response.to_string())))?;
            let api_url = api_url.as_str().unwrap();

            self.piped_api_domains.push(api_url.to_string());
            self.piped_api_domain_index = 0;
            self.piped_api_domain_index_start = 0;
        }

        self.log_send.send_log_message(format!(
            "RemoteSourceProcessor::fetch_piped_api_domains -> {:?}",
            self.get_piped_api_domain()
        ));

        Ok(())
    }
}
