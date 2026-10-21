use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TapeManifest {
    pub tape: TapeInfo,
    pub hooks: Option<Hooks>,
}

#[derive(Debug, Deserialize)]
pub struct TapeInfo {
    pub name: String,
    pub desc: String,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Hooks {
    #[serde(default)]
    pub insert: Vec<String>,

    #[serde(default)]
    pub eject: Vec<String>,
}
