//! Landlock enforcement test.
//!
//! Self-sandboxes *this process only* (this file compiles into its own test
//! binary, so other test suites are unaffected), then verifies:
//!   * writes beneath every allowed root succeed (a second root stands in for
//!     a symlinked config's resolved directory),
//!   * writes outside them fail with permission errors,
//!   * reads still work everywhere,
//!   * a thread that existed before `apply` is restricted too (ABI v8+),
//!   * only the allow-listed socket accepts connections (ABI v9+).
//!
//! Skips cleanly when the kernel cannot enforce Landlock (CI containers etc.).

use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;

use niri_center::sandbox::{Allowances, SandboxStatus, apply};
use niri_center::xdg::Places;

#[test]
fn sandbox_enforces_write_boundary() {
    // build two writable roots + a sibling that must stay read-only
    let base = match tempfile::tempdir() {
        Ok(base) => base,
        Err(error) => {
            eprintln!("no tmpdir available: {error}");
            return;
        }
    };
    let allow = base.path().join("allow");
    let resolved_target = base.path().join("dotfiles");
    let deny = base.path().join("deny");
    for dir in [&allow, &resolved_target, &deny] {
        std::fs::create_dir_all(dir).unwrap();
    }

    // sockets must exist before `apply`: binding one later is a write to /tmp
    let listed_socket = base.path().join("niri.sock");
    let unlisted_socket = base.path().join("other.sock");
    let _listed_listener = UnixListener::bind(&listed_socket).unwrap();
    let _unlisted_listener = UnixListener::bind(&unlisted_socket).unwrap();
    // the app's own folders, which `apply` creates, go here too
    let places = Places {
        preferences: base.path().join("config/niri-center/preferences"),
        snapshots: base.path().join("data/niri-center/snapshots"),
        history: base.path().join("state/niri-center/history"),
        backups: base.path().join("state/niri-center/backups"),
        probes: base.path().join("runtime/niri-center/probes"),
    };
    let allowances = Allowances {
        places: Some(places),
        sockets: vec![listed_socket.clone()],
    };

    // a thread started before `apply`, like iced's and tokio's in the GUI
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let early_target = deny.join("from-early-thread.kdl");
    let early_thread = std::thread::spawn(move || {
        go_rx.recv().unwrap();
        std::fs::write(&early_target, "nope")
    });

    let status = match apply(&[allow.clone(), resolved_target.clone()], Some(&allowances)) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("apply failed: {error}");
            return;
        }
    };

    let SandboxStatus::Active { abi, all_threads } = &status else {
        // Unsupported or opted out: nothing to assert, document why
        eprintln!("sandbox not enforced: {status:?}");
        std::mem::forget(base); // dirs were created pre-sandbox; drop would
        return; //                 try to delete from /tmp which is now denied
    };
    eprintln!("landlock active at ABI v{abi}, all threads: {all_threads}");
    let abi: u32 = abi.parse().unwrap();

    // allowed: write a file, create a subdir with another file, read back
    std::fs::write(allow.join("config.kdl"), "hi").unwrap();
    std::fs::create_dir_all(allow.join("nested")).unwrap();
    std::fs::write(allow.join("nested/other.kdl"), "yo").unwrap();
    assert_eq!(
        std::fs::read_to_string(allow.join("config.kdl")).unwrap(),
        "hi"
    );
    std::fs::write(resolved_target.join("config.kdl"), "linked").unwrap();

    // denied: writes anywhere outside the allowed roots
    let denied = std::fs::write(deny.join("config.kdl"), "nope");
    assert!(
        denied.is_err(),
        "write outside the sandbox must fail: {denied:?}"
    );
    let denied_tmp = std::fs::write(base.path().join("outside.kdl"), "nope");
    assert!(
        denied_tmp.is_err(),
        "write to the tmp parent must fail: {denied_tmp:?}"
    );

    // reads still work (root read rule)
    assert!(std::fs::read_to_string(allow.join("nested/other.kdl")).is_ok());

    go_tx.send(()).unwrap();
    let early_write = early_thread.join().unwrap();
    if abi >= 8 {
        assert!(*all_threads, "ABI v{abi} supports thread sync");
        assert!(
            early_write.is_err(),
            "a thread that predates apply must be restricted: {early_write:?}"
        );
    }

    if abi >= 9 {
        let listed = UnixStream::connect(&listed_socket);
        assert!(
            listed.is_ok(),
            "the listed socket must stay reachable: {listed:?}"
        );
        let unlisted = UnixStream::connect(&unlisted_socket);
        assert!(
            unlisted.is_err(),
            "sockets off the allow-list must be unreachable: {unlisted:?}"
        );
    }

    // removing inside the allowed roots works too; the empty dirs
    // themselves can't be rmdir'd from /tmp under the sandbox
    std::fs::remove_dir_all(allow.join("nested")).unwrap();
    std::fs::remove_file(allow.join("config.kdl")).unwrap();
    std::fs::remove_file(resolved_target.join("config.kdl")).unwrap();
    std::mem::forget(base);
}
