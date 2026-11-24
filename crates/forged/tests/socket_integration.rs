#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use forge_proto::{extract_str, Request, Response, RpcErrorCode};
use forged::{serve, DaemonConfig};

fn rpc_call(sock: &Path, id: u64, method: &str, params: &str) -> Response {
    let mut stream = UnixStream::connect(sock).expect("connect forged socket");
    let req = Request {
        id,
        method: method.to_string(),
        params_json: params.to_string(),
    };
    writeln!(stream, "{}", req.to_line()).expect("write");
    stream.flush().expect("flush");

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).expect("read");
    Response::parse_line(&line).expect("parse response")
}

#[test]
fn forged_unix_socket_search_plan_apply_and_rollback() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sock = std::env::temp_dir().join(format!("forged-int-{ts}.sock"));
    let jdir = std::env::temp_dir().join(format!("forged-int-journal-{ts}"));
    let catalog_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/catalog");

    let sock_clone = sock.clone();
    let jdir_clone = jdir.clone();
    let handle = thread::spawn(move || {
        let _ = serve(DaemonConfig {
            socket_path: sock_clone,
            catalog_dir,
            journal_dir: jdir_clone,
            idle_timeout_secs: 2,
            dry_run: true,
        });
    });

    // Wait for socket to bind
    for _ in 0..40 {
        if sock.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(sock.exists(), "forged failed to bind socket");

    // 1. Search
    match rpc_call(&sock, 1, "catalog.search", r#"{"query":"postg"}"#) {
        Response::Ok { result_json, .. } => assert!(result_json.contains("\"id\":\"postgres\"")),
        Response::Err { error, .. } => panic!("search failed: {error}"),
    }

    // 2. Missing entry returns 1001 NotFound
    match rpc_call(&sock, 2, "catalog.get", r#"{"id":"does-not-exist"}"#) {
        Response::Err { error, .. } => assert_eq!(error.code, RpcErrorCode::NotFound.as_i32()),
        Response::Ok { .. } => panic!("expected 1001 NotFound"),
    }

    // 3. Plan + Apply + Verify Parked + Rollback
    let token = match rpc_call(&sock, 3, "txn.plan", r#"{"ids":["postgres"]}"#) {
        Response::Ok { result_json, .. } => extract_str(&result_json, "token").expect("plan token"),
        Response::Err { error, .. } => panic!("plan failed: {error}"),
    };

    match rpc_call(&sock, 4, "txn.apply", &format!(r#"{{"token":"{token}"}}"#)) {
        Response::Ok { result_json, .. } => assert!(result_json.contains("completed")),
        Response::Err { error, .. } => panic!("apply failed: {error}"),
    }

    match rpc_call(&sock, 5, "system.state", "{}") {
        Response::Ok { result_json, .. } => {
            assert!(result_json.contains("\"parked\":[\"postgres\"]"));
        }
        Response::Err { error, .. } => panic!("state failed: {error}"),
    }

    match rpc_call(
        &sock,
        6,
        "txn.rollback",
        &format!(r#"{{"token":"{token}"}}"#),
    ) {
        Response::Ok { result_json, .. } => assert!(result_json.contains("rolled_back")),
        Response::Err { error, .. } => panic!("rollback failed: {error}"),
    }

    let _ = handle.join();
    let _ = std::fs::remove_dir_all(&jdir);
}
