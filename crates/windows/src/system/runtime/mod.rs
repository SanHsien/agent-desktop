pub(crate) mod com_runtime;
pub(crate) mod dpi;
pub(crate) mod hresult;
pub(crate) mod session;
#[cfg(target_os = "windows")]
pub(crate) mod thread_walk;
#[cfg(target_os = "windows")]
pub(crate) mod token_sid;
pub(crate) mod win_rect;

#[cfg(target_os = "windows")]
pub fn attach_input_desktop() {
    unsafe {
        use windows_sys::Win32::System::StationsAndDesktops::{
            CloseDesktop, OpenInputDesktop, SetThreadDesktop,
        };
        let desk = OpenInputDesktop(0, 0, 0x01FF);
        if !desk.is_null() {
            let _ = SetThreadDesktop(desk);
            CloseDesktop(desk);
        }
    }
}
