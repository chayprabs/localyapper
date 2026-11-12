//! OS permission checks: macOS Accessibility and microphone, and the
//! settings pages where the user grants them.

/// Whether this process may send key events. On macOS this is the
/// Accessibility grant; `prompt` shows the system dialog when it is off.
/// Other platforms need no grant.
pub fn accessibility_trusted(prompt: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::ax_trusted(prompt)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = prompt;
        true
    }
}

/// Microphone permission state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicPermission {
    Granted,
    Denied,
    /// The user has not been asked yet (macOS).
    NotDetermined,
}

/// Current microphone permission. Only macOS gates it per app; elsewhere
/// a denied stream surfaces as a capture error with a settings hint.
pub fn microphone_permission() -> MicPermission {
    #[cfg(target_os = "macos")]
    {
        mac::mic_status()
    }
    #[cfg(not(target_os = "macos"))]
    {
        MicPermission::Granted
    }
}

/// Ask for microphone access (macOS shows the system prompt once).
pub fn request_microphone() {
    #[cfg(target_os = "macos")]
    mac::mic_request();
}

#[cfg(target_os = "macos")]
mod mac {
    use super::MicPermission;
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
        static kAXTrustedCheckOptionPrompt: *const c_void;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *const c_void;
        fn CFRelease(cf: *const c_void);
        static kCFBooleanTrue: *const c_void;
        static kCFBooleanFalse: *const c_void;
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightPostEventAccess() -> bool;
        fn CGRequestPostEventAccess() -> bool;
    }

    /// Accessibility, plus the separate grant to post synthetic events,
    /// which macOS checks when keys are sent.
    pub fn ax_trusted(prompt: bool) -> bool {
        let ax = ax_trusted_only(prompt);
        // SAFETY: plain permission queries without arguments.
        let post = unsafe {
            if prompt {
                CGRequestPostEventAccess()
            } else {
                CGPreflightPostEventAccess()
            }
        };
        ax && post
    }

    fn ax_trusted_only(prompt: bool) -> bool {
        // SAFETY: builds a one-entry CFDictionary from static CF constants,
        // passes it to AXIsProcessTrustedWithOptions and releases it.
        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [if prompt {
                kCFBooleanTrue
            } else {
                kCFBooleanFalse
            }];
            let dict = CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                &kCFTypeDictionaryKeyCallBacks as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const c_void,
            );
            let trusted = AXIsProcessTrustedWithOptions(dict) != 0;
            if !dict.is_null() {
                CFRelease(dict);
            }
            trusted
        }
    }

    // AVFoundation: +[AVCaptureDevice authorizationStatusForMediaType:] and
    // +[AVCaptureDevice requestAccessForMediaType:completionHandler:].
    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {
        static AVMediaTypeAudio: *const c_void;
    }

    fn capture_device_class() -> Option<&'static objc2::runtime::AnyClass> {
        objc2::runtime::AnyClass::get(c"AVCaptureDevice")
    }

    pub fn mic_status() -> MicPermission {
        let Some(cls) = capture_device_class() else {
            return MicPermission::Granted;
        };
        // SAFETY: documented class method taking an AVMediaType (NSString)
        // and returning an NSInteger.
        let status: isize = unsafe {
            let media = AVMediaTypeAudio as *const objc2::runtime::AnyObject;
            objc2::msg_send![cls, authorizationStatusForMediaType: media]
        };
        match status {
            0 => MicPermission::NotDetermined,
            3 => MicPermission::Granted,
            _ => MicPermission::Denied,
        }
    }

    pub fn mic_request() {
        let Some(cls) = capture_device_class() else {
            return;
        };
        let handler = block2::RcBlock::new(|_granted: objc2::runtime::Bool| {});
        // SAFETY: documented class method; the block is copied by the callee.
        unsafe {
            let media = AVMediaTypeAudio as *const objc2::runtime::AnyObject;
            let _: () = objc2::msg_send![
                cls,
                requestAccessForMediaType: media,
                completionHandler: &*handler
            ];
        }
    }
}
