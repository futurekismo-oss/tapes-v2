use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct TapeManifest {
    pub tape: TapeInfo,
    pub hooks: Option<Hooks>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TapeInfo {
    pub name: String,
    pub desc: String,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Hooks {
    #[serde(default)]
    pub insert: Vec<String>,

    #[serde(default)]
    pub eject: Vec<String>,
}
