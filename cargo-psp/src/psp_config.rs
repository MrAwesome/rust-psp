use std::{
    fs,
    io::ErrorKind,
    process::{self},
};

const CONFIG_NAME: &str = "Psp.toml";

#[derive(serde_derive::Deserialize, Default)]
pub(crate) struct PspConfig {
    /// Title shown in the XMB menu.
    pub title: Option<String>,

    /// Path to 24bit 144x80 PNG icon shown in the XMB menu.
    pub xmb_icon_png: Option<String>,

    /// Path to animated icon shown in the XMB menu.
    ///
    /// The PSP expects a 29.97fps 144x80 PMF video file (custom Sony format).
    pub xmb_icon_pmf: Option<String>,

    /// Path to 24bit 480x272 PNG background shown in the XMB menu.
    pub xmb_background_png: Option<String>,

    /// Overlay background shown in the XMB menu.
    ///
    /// Exactly like `xmb_background_png`, but it is overlayed on top.
    pub xmb_background_overlay_png: Option<String>,

    /// Path to ATRAC3 audio file played in the XMB menu.
    ///
    /// Must be 66kbps, under 500KB and under 55 seconds.
    pub xmb_music_at3: Option<String>,

    /// Path to associated PSAR data stored in the EBOOT.
    pub psar: Option<String>,

    /// Product number of the game, in the format `ABCD-12345`.
    ///
    /// Example: UCJS-10001
    pub disc_id: Option<String>,

    /// Version of the game, e.g. "1.00".
    pub disc_version: Option<String>,

    // TODO: enum
    /// Language of the game. "JP" indicates Japanese, even though this is not
    /// the proper ISO 639 code...
    pub language: Option<String>,

    // TODO: enum
    /// Parental Control level needed to access the file. 1-11
    /// - 1 = General audience
    /// - 5 = 12 year old
    /// - 7 = 15 year old
    /// - 9 = 18 year old
    pub parental_level: Option<u32>,

    /// PSP Firmware Version required by the game (e.g. "6.61").
    pub psp_system_ver: Option<String>,

    // TODO: document values
    /// Bitmask of allowed regions. (0x8000 is region 2?)
    pub region: Option<u32>,

    /// Japanese localized title.
    pub title_jp: Option<String>,

    /// French localized title.
    pub title_fr: Option<String>,

    /// Spanish localized title.
    pub title_es: Option<String>,

    /// German localized title.
    pub title_de: Option<String>,

    /// Italian localized title.
    pub title_it: Option<String>,

    /// Dutch localized title.
    pub title_nl: Option<String>,

    /// Portugese localized title.
    pub title_pt: Option<String>,

    /// Russian localized title.
    pub title_ru: Option<String>,

    /// Used by the firmware updater to denote the firmware version it updates to.
    pub updater_version: Option<String>,
}

impl PspConfig {
    pub(crate) fn read_from_disk() -> Self {
        match fs::read(CONFIG_NAME) {
            Ok(bytes) => match toml::from_slice(&bytes) {
                Ok(config) => config,
                Err(e) => {
                    println!("Failed to read Psp.toml: {}", e);
                    println!("Please ensure that it is formatted correctly.");
                    process::exit(1);
                }
            },

            Err(e) if e.kind() == ErrorKind::NotFound => PspConfig::default(),
            Err(e) => panic!("{}", e),
        }
    }

    pub(crate) fn get_sfo_args(&self) -> impl Iterator<Item = String> {
        let raw_args = vec![
            ("-s", "DISC_ID", self.disc_id.clone()),
            ("-s", "DISC_VERSION", self.disc_version.clone()),
            ("-s", "LANGUAGE", self.language.clone()),
            ("-d",
                "PARENTAL_LEVEL",
                self.parental_level.as_ref().map(u32::to_string),
            ),
            ("-s", "PSP_SYSTEM_VER", self.psp_system_ver.clone()),
            ("-d", "REGION", self.region.as_ref().map(u32::to_string)),
            ("-s", "TITLE_0", self.title_jp.clone()),
            ("-s", "TITLE_2", self.title_fr.clone()),
            ("-s", "TITLE_3", self.title_es.clone()),
            ("-s", "TITLE_4", self.title_de.clone()),
            ("-s", "TITLE_5", self.title_it.clone()),
            ("-s", "TITLE_6", self.title_nl.clone()),
            ("-s", "TITLE_7", self.title_pt.clone()),
            ("-s", "TITLE_8", self.title_ru.clone()),
            ("-s", "UPDATER_VER", self.updater_version.clone()),
        ];

        raw_args
            .into_iter()
            // Filter through all the values that are not `None`
            .filter_map(|(f, k, v)| v.map(|v| (f, k, v)))
            // Map into 2 arguments, e.g. "-s" "NAME=VALUE"
            .flat_map(|(flag, key, value)| vec![flag.into(), format!("{}={}", key, value)])
    }
}
