use crate::client::client::State;
use crate::util::util;
use serde_json;
use std::fs::File;
use std::io::Write;

pub struct Command {
    pub name: &'static str,
    pub executor: fn(argv: &String) -> i32,
}

fn cmd_init(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, bundle, exit_fifo_dir) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["bundle"].as_str().unwrap().to_string(),
        values["exit_fifo_dir"].as_str().unwrap().to_string(),
    );
    util::shim_v2_init(container_id, bundle, exit_fifo_dir);
    1
}

fn cmd_create(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, bundle, terminal, stdin, stdout, stderr) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["bundle"].as_str().unwrap().to_string(),
        values["terminal"].as_bool().unwrap_or_default(),
        values["stdin"].as_str().unwrap_or_default().to_string(),
        values["stdout"].as_str().unwrap_or_default().to_string(),
        values["stderr"].as_str().unwrap_or_default().to_string(),
    );

    util::shim_v2_new(container_id.clone(), util::get_addr(bundle.clone()));

    let mut pid = -1;
    util::shim_v2_create(
        container_id,
        bundle,
        terminal,
        stdin,
        stdout,
        stderr,
        &mut pid,
    );
    println!("create gets pid {}", pid);
    1
}

fn cmd_start(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, exec_id, bundle) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["exec_id"].as_str().unwrap_or_default().to_string(),
        values["bundle"].as_str().unwrap_or_default().to_string(),
    );
    if !bundle.eq(&String::default()) {
        util::shim_v2_new(container_id.clone(), util::get_addr(bundle.clone()));
    } else {
        let mut bundle = String::from("/var/lib/isulad/engines/remote/");

        bundle.push_str(values["container_id"].as_str().unwrap());

        if util::shim_v2_new(container_id.clone(), util::get_addr(bundle)) != 0 {
            return -1;
        }
    }

    let mut pid = -1;

    let ret = util::shim_v2_start(container_id, exec_id, &mut pid);

    if ret != -1 && !bundle.eq(&String::default()) {
        let mut pid_file_path = String::from(bundle);
        pid_file_path.push_str("/shim_v2_pid");
        let mut file = File::create(pid_file_path).expect("create file failed");
        file.write_all(pid.to_string().as_bytes())
            .expect("write failed");
        1
    } else {
        0
    }
}

fn cmd_exec(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, exec_id, terminal, stdin, stdout, stderr, spec) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["exec_id"].as_str().unwrap_or_default().to_string(),
        values["terminal"].as_bool().unwrap_or_default(),
        values["stdin"].as_str().unwrap_or_default().to_string(),
        values["stdout"].as_str().unwrap_or_default().to_string(),
        values["stderr"].as_str().unwrap_or_default().to_string(),
        values["spec"].as_str().unwrap_or_default().to_string(),
    );

    let mut bundle = String::from("/var/lib/isulad/engines/remote/");

    bundle.push_str(values["container_id"].as_str().unwrap());

    if util::shim_v2_new(container_id.clone(), util::get_addr(bundle)) != 0 {
        return -1;
    }
    util::shim_v2_exec(container_id, exec_id, terminal, stdin, stdout, stderr, spec)
}

fn cmd_resize(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, exec_id, height, width) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["exec_id"].as_str().unwrap_or_default().to_string(),
        values["height"].as_u64().unwrap_or_default() as u32,
        values["width"].as_u64().unwrap_or_default() as u32,
    );
    let mut bundle = String::from("/var/lib/isulad/engines/remote/");

    bundle.push_str(values["container_id"].as_str().unwrap());

    if util::shim_v2_new(container_id.clone(), util::get_addr(bundle)) != 0 {
        return -1;
    }
    util::shim_v2_resize_pty(container_id, exec_id, height, width)
}

fn cmd_kill(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, exec_id, signal, all) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["exec_id"].as_str().unwrap_or_default().to_string(),
        values["signal"].as_u64().unwrap_or_default() as u32,
        values["all"].as_bool().unwrap_or_default(),
    );
    let mut bundle = String::from("/var/lib/isulad/engines/remote/");
    bundle.push_str(values["container_id"].as_str().unwrap());

    if util::shim_v2_new(container_id.clone(), util::get_addr(bundle)) != 0 {
        return -1;
    }
    util::shim_v2_kill(container_id, exec_id, signal, all)
}

fn cmd_wait(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, exec_id) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["exec_id"].as_str().unwrap_or_default().to_string(),
    );
    let mut exit_status = -1;
    let mut bundle = String::from("/var/lib/isulad/engines/remote/");

    bundle.push_str(values["container_id"].as_str().unwrap());

    if util::shim_v2_new(container_id.clone(), util::get_addr(bundle)) != 0 {
        return -1;
    }
    util::shim_v2_wait(container_id, exec_id, &mut exit_status)
}

fn cmd_status(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let container_id = values["container_id"].as_str().unwrap().to_string();

    let mut bundle = String::from("/var/lib/isulad/engines/remote/");
    bundle.push_str(values["container_id"].as_str().unwrap());
    util::shim_v2_new(container_id.clone(), util::get_addr(bundle));
    let mut state = State::default();

    let ret = util::shim_v2_state(container_id, &mut state);
    if ret == 0 {
        let res = serde_json::to_string(&state);
        match res {
            Ok(v) => println!("{}", v),
            Err(e) => println!("{}", e),
        }
        0
    } else {
        -1
    }
}

fn cmd_delete(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let (container_id, exec_id) = (
        values["container_id"].as_str().unwrap().to_string(),
        values["exec_id"].as_str().unwrap_or_default().to_string(),
    );
    let mut resp = util::DeleteResponse::new();

    let mut bundle = String::from("/var/lib/isulad/engines/remote/");
    bundle.push_str(values["container_id"].as_str().unwrap());

    util::shim_v2_new(container_id.clone(), util::get_addr(bundle));
    util::shim_v2_delete(container_id, exec_id, &mut resp);
    let output = serde_json::to_string(&resp).unwrap();
    std::io::stdout()
        .write(format!("{}", output).as_bytes())
        .unwrap();
    1
}

fn cmd_shutdown(argv: &String) -> i32 {
    let values: serde_json::Value = serde_json::from_str(&argv).unwrap();
    let container_id = values["container_id"].as_str().unwrap().to_string();
    let mut bundle = String::from("/var/lib/isulad/engines/remote/");
    bundle.push_str(values["container_id"].as_str().unwrap());

    util::shim_v2_new(container_id.clone(), util::get_addr(bundle));

    util::shim_v2_shutdown(container_id)
}

pub const GLOBAL_COMMANDS: [Command; 10] = [
    Command {
        name: "init",
        executor: cmd_init,
    },
    Command {
        name: "create",
        executor: cmd_create,
    },
    Command {
        name: "start",
        executor: cmd_start,
    },
    Command {
        name: "exec",
        executor: cmd_exec,
    },
    Command {
        name: "resize",
        executor: cmd_resize,
    },
    Command {
        name: "status",
        executor: cmd_status,
    },
    Command {
        name: "wait",
        executor: cmd_wait,
    },
    Command {
        name: "delete",
        executor: cmd_delete,
    },
    Command {
        name: "shutdown",
        executor: cmd_shutdown,
    },
    Command {
        name: "kill",
        executor: cmd_kill,
    },
];
