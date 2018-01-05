#[macro_use]
extern crate criterion;
#[macro_use]
extern crate serde_json;
extern crate serde;
extern crate xi_core_lib;

mod process;

use criterion::*;
use process::XiCore;
use serde::de::{Deserialize};
use xi_core_lib as xi_core;
use xi_core_lib::rpc;

fn new_core() -> XiCore {
    let mut xi = XiCore::new("xi-core");
    xi.send(rpc::CoreNotification::ClientStarted {
        config_dir: None,
        client_extras_dir: None,
    }).unwrap();

    xi
}

fn bench_simple_new_view(b: &mut Bencher) {
    let mut xi = new_core();
    b.iter(|| {
        xi.sync_request(rpc::CoreRequest::NewView { file_path: None })
          .expect("Error setting up NewView");
    })
}

fn bench_simple_edits(b: &mut Bencher) {
    let mut xi = new_core();
    let new_view = xi
        .sync_request(rpc::CoreRequest::NewView { file_path: None })
        .expect("Error setting up NewView")
        .get("result")
        .unwrap()
        .clone();
    let viewid = xi_core::ViewId::deserialize(new_view).unwrap();
    b.iter(|| {
        xi.send(rpc::CoreNotification::Edit(rpc::EditCommand {
            view_id: viewid,
            cmd: rpc::EditNotification::Insert { chars: "a".into() },
        })).unwrap();
    })
}

fn bench(c: &mut Criterion) {
    c.bench_function("simple_new_view", bench_simple_new_view);
    c.bench_function("simple_edits", bench_simple_edits);
}

criterion_group!(benches, bench);
criterion_main!(benches);