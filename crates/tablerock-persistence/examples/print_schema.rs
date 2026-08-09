
fn main() {
  let path = std::env::temp_dir().join("schema-check2.db");
  let _ = std::fs::remove_file(&path);
  let a = tablerock_persistence::PersistenceActor::open(&path).unwrap();
  a.shutdown().unwrap();
  let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
  rt.block_on(async {
    let db = turso::Builder::new_local(path.to_str().unwrap()).build().await.unwrap();
    let c = db.connect().unwrap();
    for name in ["saved_profiles","query_history","saved_queries"] {
      let mut rows = c.query("SELECT sql FROM sqlite_master WHERE name=?1", (name,)).await.unwrap();
      while let Some(r) = rows.next().await.unwrap() {
        let s: String = r.get(0).unwrap();
        println!("=== {name}\n{s}\n");
      }
    }
  });
}
