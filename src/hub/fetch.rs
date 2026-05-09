use std::time::Duration;

use crate::error::{VexError, VexResult};
use crate::hub::join_url;
use crate::hub::types::HubIndex;
use crate::remote::PublishedConfig;

const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(HTTP_TIMEOUT).build()
}

pub fn fetch_index(base_url: &str) -> VexResult<HubIndex> {
    let url = join_url(base_url, "index.json");
    let resp = agent().get(&url).call().map_err(|e| map_ureq(&url, e))?;
    let body = resp.into_string().map_err(|e| VexError::HubRequestFailed {
        url: url.clone(),
        status: 0,
        body: format!("read body failed: {}", e),
    })?;
    serde_json::from_str(&body).map_err(|e| VexError::HubIndexParseFailed { source: e })
}

pub fn fetch_published_config(
    base_url: &str,
    id: &str,
    name: &str,
    tag: &str,
) -> VexResult<PublishedConfig> {
    let path = format!("configs/{}/{}/{}.json", id, name, tag);
    let url = join_url(base_url, &path);
    match agent().get(&url).call() {
        Ok(resp) => {
            let body = resp.into_string().map_err(|e| VexError::HubRequestFailed {
                url: url.clone(),
                status: 0,
                body: format!("read body failed: {}", e),
            })?;
            serde_json::from_str(&body).map_err(|e| VexError::ConfigParseFailed { source: e })
        }
        Err(ureq::Error::Status(404, _)) => Err(VexError::HubEntryNotFound {
            id: id.into(),
            name: name.into(),
            tag: tag.into(),
        }),
        Err(e) => Err(map_ureq(&url, e)),
    }
}

fn map_ureq(url: &str, e: ureq::Error) -> VexError {
    match e {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            VexError::HubRequestFailed {
                url: url.into(),
                status: code,
                body,
            }
        }
        ureq::Error::Transport(t) => VexError::HubRequestFailed {
            url: url.into(),
            status: 0,
            body: format!("transport error: {}", t),
        },
    }
}
