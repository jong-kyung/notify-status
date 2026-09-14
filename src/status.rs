//! Public response types and constructors.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[napi(string_enum = "camelCase")]
pub enum Authorization {
    Granted,
    Denied,
    NotDetermined,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn unsupported(platform: impl Into<String>, reason: Reason) -> Self {
        Self {
            authorization: Authorization::Unsupported,
            do_not_disturb: false,
            platform: platform.into(),
            reason: Some(reason),
        }
    }
}
