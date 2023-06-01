use serde::Deserialize;
use serde_this_or_that::as_bool;

structstruck::strike! {
    #[strikethrough[derive(Debug, Deserialize)]]
    pub struct LastFM {
        #[serde(rename = "recenttracks")]
        pub recent: struct {
            #[serde(rename = "track")]
            pub tracks: Vec<pub struct Track {
                pub artist: struct {
                    pub mbid: String,
                    #[serde(rename = "#text")]
                    pub text: String,
                },
                pub streamable: String,
                pub image: Vec<pub struct {
                    pub size: String,
                    #[serde(rename = "#text")]
                    pub text: String,
                }>,
                pub mbid: String,
                pub album: struct {
                    pub mbid: String,
                    #[serde(rename = "#text")]
                    pub text: String,
                },
                pub name: String,
                #[serde(rename = "@attr")]
                pub attr: Option<pub struct {
                    #[serde(deserialize_with = "as_bool")]
                    pub nowplaying: bool,
                }>,
                pub url: String,
                pub date: Option<pub struct {
                    pub uts: String,
                    #[serde(rename = "#text")]
                    pub text: String,
                }>,
            }>,
            #[serde(rename = "@attr")]
            pub attr: struct Attributes {
                pub user: String,
                #[serde(rename = "totalPages")]
                pub total_pages: String,
                pub page: String,
                pub total: String,
                #[serde(rename = "perPage")]
                pub per_page: String,
            },
        },
    }
}
