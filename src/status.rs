//! Public response types. Filled out in U2.

#[napi(string_enum = "camelCase")]
pub enum Authorization {
    Granted,
    Denied,
    NotDetermined,
    Unsupported,
}

#[napi(string_enum = "camelCase")]
pub enum Reason {
    NoBundleId,
    NoAumid,
    UnsupportedPlatform,
    InternalError,
}

#[napi(object)]
pub struct NotificationStatus {
    pub authorization: Authorization,
    pub do_not_disturb: bool,
    pub platform: String,
    pub reason: Option<Reason>,
}

impl NotificationStatus {
    pub fn unsupported(platform: String, reason: Reason) -> Self {
        Self {
            authorization: Authorization::Unsupported,
            do_not_disturb: false,
            platform,
            reason: Some(reason),
        }
    }
}
