/// The Echo messages urls
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum CollectorUrl {
    /// The stage url (https://echocollector-stage.kroger.com/echo/messages)
    Stage,
    /// The prod url (https://echocollector.kroger.com/echo/messages)
    Prod,
}

impl Default for CollectorUrl {
    fn default() -> Self {
        CollectorUrl::Stage
    }
}

impl CollectorUrl {
    /// Convert the enum to a str
    pub fn as_str(self) -> &'static str {
        match self {
            CollectorUrl::Stage => "https://echocollector-stage.kroger.com/echo/messages",
            CollectorUrl::Prod => "https://echocollector.kroger.com/echo/messages",
        }
    }
}