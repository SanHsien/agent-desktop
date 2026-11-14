use super::*;
use crate::system::window::window_enum::window_is_responsive;

#[test]
#[ignore = "runs only in the re-executed clipboard holder host process"]
fn clipboard_holder_host_process_entry() {
    assert!(
        is_clipboard_holder_host(),
        "the clipboard holder entry must not run without the host flag"
    );
    run_as_clipboard_holder_host();
}

#[test]
fn delayed_owner_advertises_format_then_stops_pumping() {
    let _lock = clipboard_test_lock();
    let owner = DelayedClipboardOwner::create().expect("delayed owner starts");
    assert!(
        owner.format_available(),
        "delay-rendered CF_UNICODETEXT must be advertised"
    );
    assert!(
        owner.owner_is_self(),
        "GetClipboardOwner must name the delayed-owner window"
    );
    assert!(
        !window_is_responsive(owner.handle() as _),
        "the delayed owner must stop pumping after advertising"
    );
}

#[test]
fn contending_holder_is_a_second_process_that_blocks_open() {
    let _lock = clipboard_test_lock();
    let mut holder = ContendingClipboardHolder::spawn().expect("holder starts");
    assert_ne!(
        holder.process_id(),
        std::process::id(),
        "contention requires a genuine second process"
    );
    assert_ne!(holder.open_clipboard_window(), 0);
    assert!(
        !try_open_clipboard(None),
        "OpenClipboard must fail while the holder keeps the clipboard open"
    );
    holder.release().expect("holder releases on request");
    assert!(
        open_clipboard_once_free(None),
        "OpenClipboard must succeed after the holder releases"
    );
    close_clipboard();
}

#[test]
fn delayed_owner_clears_delay_format_on_drop() {
    let _lock = clipboard_test_lock();
    {
        let owner = DelayedClipboardOwner::create().expect("delayed owner starts");
        assert!(owner.format_available());
    }
    assert!(
        !unsafe { windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable(13) != 0 },
        "dropping the delayed owner must clear the advertised delay-rendered format"
    );
}

/// The release a caller waits for is the clipboard coming free. A host whose
/// process outlives that moment must not turn a completed release into a
/// failure: under CPU load the OS was measured taking from seconds to over a
/// minute to finish tearing a host down after it had closed the clipboard.
#[test]
fn a_holder_whose_process_outlives_its_release_still_reports_the_release() {
    let _lock = clipboard_test_lock();
    let mut holder = ContendingClipboardHolder::spawn_lingering(Duration::from_secs(20))
        .expect("the lingering holder starts");
    assert!(
        !try_open_clipboard(None),
        "OpenClipboard must fail while the holder keeps the clipboard open"
    );
    holder
        .release()
        .expect("the clipboard coming free is the release, whatever the host process does next");
    assert!(
        open_clipboard_once_free(None),
        "OpenClipboard must succeed once the holder has let go, while its process is still alive"
    );
    close_clipboard();
}

/// Clipboard listeners on a real desktop (history, remote-session sync) open
/// the clipboard for a moment after every write, so an open that needs the
/// clipboard has to outwait one instead of failing on a single attempt.
///
/// The opener holds the clipboard until this thread reports that its single
/// attempt has been made, so that attempt meets a held clipboard however
/// late this thread is scheduled. Every open is closed and the opener joined
/// before anything is asserted, so a failure leaves the clipboard free.
#[test]
fn an_open_that_needs_the_clipboard_outwaits_a_transient_opener() {
    let _lock = clipboard_test_lock();
    let (opened, wait_opened) = channel();
    let (single_attempt_made, wait_single_attempt) = channel::<()>();
    let opener = spawn(move || {
        let window = message_only_window();
        let held = !window.is_null() && open_clipboard_once_free(Some(window as isize));
        let _ = opened.send(held);
        let _ = wait_single_attempt.recv();
        std::thread::sleep(Duration::from_millis(200));
        if held {
            close_clipboard();
        }
        unsafe { DestroyWindow(window) };
    });
    let held = wait_opened.recv().expect("the opener reports");
    let single_attempt_won = try_open_clipboard(None);
    if single_attempt_won {
        close_clipboard();
    }
    let _ = single_attempt_made.send(());
    let waited_open_won = open_clipboard_once_free(None);
    if waited_open_won {
        close_clipboard();
    }
    opener.join().expect("the opener thread ends");

    assert!(held, "the transient opener takes the clipboard");
    assert!(
        !single_attempt_won,
        "a single attempt loses to the opener, which is what makes the wait necessary"
    );
    assert!(
        waited_open_won,
        "the open must succeed once the transient opener lets go"
    );
}
