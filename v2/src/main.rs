#[tokio::main(worker_threads = 2)]
async fn main() {
    if let Err(e) = tezbar_extension_sdk::serve(modify_hash_v2::Adapter::default()).await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
