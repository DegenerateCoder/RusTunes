pub struct RemoteSourceProcessor {
    pub piped_api_domains: Vec<String>,
    pub piped_api_domain_index: usize,
    pub invidious_api_domains: Vec<String>,
    pub invidious_api_domain_index: usize,
    pub duration_limit: u64,
}

#[derive(Clone)]
pub struct Remote {
    pub url: String,
    pub video_id: String,
    pub audio_stream_url: String,
    pub title: String,
    pub length: u64,
}

#[derive(Clone)]
pub struct Local {}

#[derive(Clone)]
pub enum Source {
    Remote(Remote),
    _Local(Local),
}

impl Source {
    pub fn new_remote(path: &str) -> Self {
        if path.starts_with("https://") || path.contains("/watch?") {
            Source::Remote(Remote::new(path))
        } else {
            panic!("Not a valid url: {path}");
        }
    }
}

impl Remote {
    pub fn new(path: &str) -> Self {
        Remote {
            url: path.to_string(),
            video_id: Remote::url_into_video_id(path),
            audio_stream_url: "".to_string(),
            title: "".to_string(),
            length: 0,
        }
    }

    pub fn url_into_video_id(url: &str) -> String {
        let split = url.split("v=");
        let id = split.last().unwrap().to_string();

        id
    }

    pub fn url_into_playlist_id(url: &str) -> String {
        let split = url.split("list=");
        let id = split.last().unwrap().to_string();

        id
    }
}

impl RemoteSourceProcessor {
    pub fn set_audio_url_title(&self, source: &mut Remote) {
        let request_url = format!(
            "{}/streams/{}",
            self.piped_api_domains[self.piped_api_domain_index], &source.video_id
        );
        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();
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
    }

    pub fn get_video_genre(&self, source: &Remote) -> String {
        let request_url = format!(
            "{}/api/v1/videos/{}",
            self.invidious_api_domains[self.invidious_api_domain_index], &source.video_id
        );
        let response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();
        let genre: String = response.get("genre").unwrap().as_str().unwrap().to_string();

        genre
    }

    pub fn get_related_video_url(&self, video_id: &str, played_video_ids: &Vec<String>) -> Source {
        let request_url = format!(
            "{}/streams/{}",
            self.piped_api_domains[self.piped_api_domain_index], video_id
        );
        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();
        let related_streams: &mut Vec<serde_json::Value> = response
            .get_mut("relatedStreams")
            .unwrap()
            .as_array_mut()
            .unwrap();

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
            ) {
                //println!("Next to play: {related_video_url} <- from {video_id}");
                return Source::new_remote(related_video_url);
            }
        }
        panic!("No related videos found");
    }

    fn check_filters_for_related_video_url(
        &self,
        video_url: &str,
        stream_json: &serde_json::Value,
        played_video_ids: &Vec<String>,
    ) -> bool {
        let new_remote_src = Source::new_remote(video_url);
        match new_remote_src {
            Source::Remote(remote_src) => {
                let video_id = &remote_src.video_id;
                if played_video_ids.contains(video_id) {
                    return false;
                } else if stream_json.get("duration").unwrap().as_u64().unwrap_or(self.duration_limit+1)// .unwrap()
                    > self.duration_limit
                {
                    return false;
                } else if !self.get_video_genre(&remote_src).contains("Music") {
                    return false;
                }
            }
            _ => panic!(),
        }
        true
    }

    pub fn playlist_to_remote_vec(&self, playlist_id: &str) -> Vec<Source> {
        let mut playlist = Vec::new();
        let request_url = format!(
            "{}/playlists/{}",
            self.piped_api_domains[self.piped_api_domain_index], playlist_id
        );

        let mut response: serde_json::Value = reqwest::blocking::get(&request_url)
            .unwrap()
            .json()
            .unwrap();

        loop {
            let related_streams: &mut Vec<serde_json::Value> = response
                .get_mut("relatedStreams")
                .unwrap()
                .as_array_mut()
                .unwrap();

            for stream in related_streams {
                let url = stream.get("url").unwrap().as_str().unwrap().to_string();
                let video_id = Remote::url_into_video_id(&url);

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
                self.piped_api_domains[self.piped_api_domain_index],
                playlist_id,
                urlencoding::encode(&nextpage)
            );

            response = reqwest::blocking::get(&request_url)
                .unwrap()
                .json()
                .unwrap();
        }

        playlist
    }
}