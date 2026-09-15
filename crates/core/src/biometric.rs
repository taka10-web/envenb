//! Human presence check via the macOS LocalAuthentication framework.
//!
//! Used before an action that widens access to a stored secret. Touch ID is
//! offered when available; otherwise (and on failure) the system falls back to
//! the login password, which is why the API is a single "prove you are the
//! owner of this Mac" call rather than a Touch-ID-specific one.

/// Ask the OS to confirm the user, showing `reason` in the system prompt.
///
/// Returns `Ok(())` only when the OS reports success. A cancelled prompt, a
/// failed match and an unavailable policy are all errors: callers must treat
/// anything but `Ok` as "not authorised".
#[cfg(target_os = "macos")]
pub fn authenticate(reason: &str) -> Result<(), String> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::msg_send;
    use objc2_foundation::NSString;
    use std::sync::mpsc;

    // LAPolicy::DeviceOwnerAuthentication == 2: biometry, watch, or password.
    const POLICY_DEVICE_OWNER_AUTHENTICATION: isize = 2;

    unsafe {
        let context: Retained<AnyObject> = {
            // Resolved at runtime: the class is absent when the
            // LocalAuthentication framework is not loaded (headless CI), and
            // that must be an error rather than a panic.
            let Some(cls) = objc2::runtime::AnyClass::get(c"LAContext") else {
                return Err("LocalAuthentication is unavailable".into());
            };
            let obj: *mut AnyObject = msg_send![cls, new];
            match Retained::from_raw(obj) {
                Some(c) => c,
                None => return Err("LocalAuthentication is unavailable".into()),
            }
        };

        let ns_reason = NSString::from_str(reason);
        let (tx, rx) = mpsc::channel::<bool>();

        // The completion handler fires on an internal queue, so hand the
        // result back over a channel and block until it arrives.
        let block = block2::RcBlock::new(move |success: objc2::runtime::Bool, _error: *mut AnyObject| {
            let _ = tx.send(success.as_bool());
        });

        let _: () = msg_send![
            &*context,
            evaluatePolicy: POLICY_DEVICE_OWNER_AUTHENTICATION,
            localizedReason: &*ns_reason,
            reply: &*block,
        ];

        match rx.recv_timeout(std::time::Duration::from_secs(120)) {
            Ok(true) => Ok(()),
            Ok(false) => Err("authentication was declined".into()),
            Err(_) => Err("authentication timed out".into()),
        }
    }
}

/// Non-macOS builds have no supported prompt, so the action is refused.
#[cfg(not(target_os = "macos"))]
pub fn authenticate(_reason: &str) -> Result<(), String> {
    Err("device owner authentication is only available on macOS".into())
}


/// Proof that the device owner approved a specific action.
///
/// The only way to obtain one is [`DeviceOwnerApproval::prompt`], which returns
/// `Err` unless the OS confirmed the user. Functions that widen access to a
/// secret take this by value, so the check cannot be forgotten at a call site
/// and cannot be faked from another crate.
#[derive(Debug)]
#[non_exhaustive]
pub struct DeviceOwnerApproval {
    _private: (),
}

impl DeviceOwnerApproval {
    /// Show the system prompt and, on success, mint the proof.
    pub fn prompt(reason: &str) -> Result<Self, String> {
        authenticate(reason)?;
        Ok(Self { _private: () })
    }
}
