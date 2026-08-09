fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("tablerock-sample.db");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        tablerock_engine::ensure_sample_sqlite_database(&path).await.unwrap();
        // reopen and checkpoint by drop after query
        let session = tablerock_engine::SqliteSession::connect(&path).await.unwrap();
        drop(session);
        println!("wrote {}", path.display());
    });
}
