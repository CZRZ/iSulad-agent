use crate::client::client::State as client_state;
use crate::client::client::Status as client_status;
use crate::client::client::{del_conn, get_conn, new_conn};
use crate::util::util::Status::{
    CreatedStatus, DeletedStatus, PauseStatus, PausingStatus, RunningStatus, StoppedStatus,
    UnknownStatus,
};
use serde::Serialize;
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::os::raw::{c_char, c_int, c_uint};

const SOCKET_FILE: &'static str = "/shim_v2_socket";

pub fn to_string(x: *const c_char) -> String {
    unsafe {
        if x.is_null() {
            "".to_string()
        } else {
            CStr::from_ptr(x).to_str().unwrap_or_default().to_string()
        }
    }
}

pub fn get_addr(bundle: String) -> String {
    let mut socket_file_path = String::from(bundle);
    socket_file_path.push_str(SOCKET_FILE);
    let mut file = File::open(socket_file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents);
    assert_eq!(contents.contains("unix"), true);
    contents
}

pub fn shim_v2_init(
    container_id: String,
    bundle: String,
    exit_fifo_dir: String,
) -> std::io::Result<()> {
    let output = std::process::Command::new("/usr/local/bin/containerd-shim-runc-v2")
        .current_dir(bundle.clone())
        .env("EXIT_FIFO_DIR", exit_fifo_dir)
        .arg("--id")
        .arg(container_id)
        .arg("--namespace")
        .arg("isula")
        .arg("start")
        .output();

    let addr = match String::from_utf8(output.unwrap().stdout) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
    // save socket addr to disk
    let mut socket_file_path = String::from(bundle);
    socket_file_path.push_str(SOCKET_FILE);
    let mut file = File::create(socket_file_path).expect("create file failed");
    file.write_all(addr.as_bytes()).expect("write failed");
    Ok(())
}

pub fn shim_v2_new(container_id: String, addr: String) -> c_int {
    if let Err(e) = new_conn(&container_id, &addr) {
        println!("remote-shim-v2::new::{}:: failed, {}.", container_id, e);
        return -1;
    }
    0
}

#[no_mangle]
pub fn shim_v2_close(container_id: *const c_char) -> c_int {
    let r_container_id = to_string(container_id);
    println!("remote-shim-v2::close::{}::", r_container_id);
    del_conn(&r_container_id);
    0
}

#[no_mangle]
pub fn shim_v2_create(
    container_id: String,
    bundle: String,
    terminal: bool,
    stdin: String,
    stdout: String,
    stderr: String,
    pid: &mut c_int,
) -> c_int {
    println!(
        "remote-shim-v2::create::{}:: [{} {} {} {} {}]",
        container_id, bundle, terminal, stdin, stdout, stderr
    );
    get_conn(&container_id)
        .and_then(|client| {
            client
                .create(&container_id, &bundle, terminal, &stdin, &stdout, &stderr)
                .map(|process_pid| {
                    *pid = process_pid;
                    println!("remote-shim-v2::create::{}:: done.", container_id);
                    0
                })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::create::{}:: failed, {}.", container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_start(container_id: String, exec_id: String, pid: &mut c_int) -> c_int {
    println!("remote-shim-v2::start::{}:: [{}]", container_id, exec_id);
    get_conn(&container_id)
        .and_then(|client| {
            client.start(&container_id, &exec_id).map(|process_pid| {
                *pid = process_pid;
                println!("remote-shim-v2::start::{}:: done.", container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::start::{}:: failed, {}.", container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_kill(container_id: String, exec_id: String, signal: u32, all: bool) -> c_int {
    println!("remote-shim-v2::kill::{}:: [{}]", container_id, exec_id);
    get_conn(&container_id)
        .and_then(|client| {
            client.kill(&container_id, &exec_id, signal, all).map(|_| {
                println!("remote-shim-v2::kill::{}:: done.", container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::kill::{}:: failed, {}.", container_id, e);
            -1
        })
}

#[repr(C)]
#[derive(Serialize)]
pub struct DeleteResponse {
    exit_status: c_uint,
    pid: c_uint,
}

impl DeleteResponse {
    pub fn new() -> DeleteResponse {
        DeleteResponse {
            exit_status: 0,
            pid: 0,
        }
    }
}

#[no_mangle]
pub fn shim_v2_delete(container_id: String, exec_id: String, resp: &mut DeleteResponse) -> c_int {
    println!("remote-shim-v2::delete::{}:: [{}]", container_id, exec_id);
    get_conn(&container_id)
        .and_then(|client| {
            client.delete(&container_id, &exec_id).map(|response| {
                resp.exit_status = response.exit_status;
                resp.pid = response.pid;
                println!("remote-shim-v2::delete::{}:: done.", container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::delete::{}:: failed, {}.", container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_shutdown(container_id: String) -> c_int {
    println!("remote-shim-v2::shutdown::{}::", container_id);
    get_conn(&container_id)
        .and_then(|client| {
            client.shutdown(&container_id).map(|_| {
                println!("remote-shim-v2::shutdown::{}:: done.", container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!(
                "remote-shim-v2::shutdown::{}:: failed, {}.",
                container_id, e
            );
            -1
        })
}

#[no_mangle]
pub fn shim_v2_exec(
    container_id: String,
    exec_id: String,
    terminal: bool,
    stdin: String,
    stdout: String,
    stderr: String,
    spec: String,
) -> c_int {
    let r_spec = spec.as_bytes();

    println!(
        "remote-shim-v2::exec::{}:: [{} {} {} {} {}]",
        container_id, exec_id, terminal, stdin, stdout, stderr
    );
    get_conn(&container_id)
        .and_then(|client| {
            client
                .exec(
                    &container_id,
                    &exec_id,
                    terminal,
                    &stdin,
                    &stdout,
                    &stderr,
                    r_spec,
                )
                .map(|_| {
                    println!("remote-shim-v2::exec::{}:: done.", container_id);
                    0
                })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::exec::{}:: failed, {}.", container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_resize_pty(container_id: String, exec_id: String, height: u32, width: u32) -> c_int {
    println!(
        "remote-shim-v2::resize_pty::{}:: [{}]",
        container_id, exec_id
    );
    get_conn(&container_id)
        .and_then(|client| {
            client
                .resize_pty(&container_id, &exec_id, height, width)
                .map(|_| {
                    println!("remote-shim-v2::resize_pty::{}:: done.", container_id);
                    0
                })
        })
        .unwrap_or_else(|e| {
            println!(
                "remote-shim-v2::resize_pty::{}:: failed, {}.",
                container_id, e
            );
            -1
        })
}

#[no_mangle]
pub fn shim_v2_pause(container_id: *const c_char) -> c_int {
    let r_container_id = to_string(container_id);
    println!("remote-shim-v2::pause::{}::", r_container_id);
    get_conn(&r_container_id)
        .and_then(|client| {
            client.pause(&r_container_id).map(|_| {
                println!("remote-shim-v2::pause::{}:: done.", r_container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::pause::{}:: failed, {}.", r_container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_resume(container_id: *const c_char) -> c_int {
    let r_container_id = to_string(container_id);
    println!("remote-shim-v2::resume::{}::", r_container_id);
    get_conn(&r_container_id)
        .and_then(|client| {
            client.resume(&r_container_id).map(|_| {
                println!("remote-shim-v2::resume::{}:: done.", r_container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!(
                "remote-shim-v2::resume::{}:: failed, {}.",
                r_container_id, e
            );
            -1
        })
}

#[repr(C)]
pub enum Status {
    UnknownStatus,
    CreatedStatus,
    RunningStatus,
    StoppedStatus,
    DeletedStatus,
    PauseStatus,
    PausingStatus,
}

impl Status {
    fn new(in_obj: client_status) -> Status {
        match in_obj {
            client_status::UnknownStatus => UnknownStatus,
            client_status::CreatedStatus => CreatedStatus,
            client_status::RunningStatus => RunningStatus,
            client_status::StoppedStatus => StoppedStatus,
            client_status::DeletedStatus => DeletedStatus,
            client_status::PauseStatus => PauseStatus,
            client_status::PausingStatus => PausingStatus,
        }
    }
}

#[no_mangle]
pub fn shim_v2_state(container_id: String, state: &mut client_state) -> c_int {
    get_conn(&container_id)
        .and_then(|client| {
            client.state(&container_id).map(|container_state| {
                state.copy(container_state);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::state::{}:: failed, {}.", container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_pids(container_id: *const c_char, pid: &mut c_int) -> c_int {
    let r_container_id = to_string(container_id);
    println!("in rutst::shim_v2_pids::{}:: start.", r_container_id);
    get_conn(&r_container_id)
        .and_then(|client| {
            client.pids(&r_container_id).map(|process_pid| {
                *pid = process_pid;
                println!("in rust::shim_v2_pids::{}:: done", r_container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("in rust::shim_v2_pids::{}:: failed, {}", r_container_id, e);
            -1
        })
}

#[no_mangle]
pub fn shim_v2_wait(container_id: String, exec_id: String, exit_status: &mut c_int) -> c_int {
    println!("remote-shim-v2::wait::{}:: [{}]", container_id, exec_id);
    get_conn(&container_id)
        .and_then(|client| {
            client.wait(&container_id, &exec_id).map(|exit_code| {
                *exit_status = exit_code;
                println!("remote-shim-v2::wait::{}:: done.", container_id);
                0
            })
        })
        .unwrap_or_else(|e| {
            println!("remote-shim-v2::wait::{}:: failed, {}.", container_id, e);
            -1
        })
}
