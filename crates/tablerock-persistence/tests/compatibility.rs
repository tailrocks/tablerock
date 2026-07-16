use std::{fs, path::PathBuf};

fn temporary_database() -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-turso-compatibility-{}.db",
        std::process::id()
    ))
}

#[tokio::test(flavor = "current_thread")]
async fn foreign_keys_and_ordinary_rollback_work_without_experimental_features() {
    let path = temporary_database();
    let _ = fs::remove_file(&path);
    let database = turso::Builder::new_local(path.to_str().unwrap())
        .build()
        .await
        .unwrap();
    let mut connection = database.connect().unwrap();
    connection
        .pragma_update("foreign_keys", "ON")
        .await
        .unwrap();
    connection
        .execute_batch(
            "CREATE TABLE parent(id INTEGER PRIMARY KEY);\n\
             CREATE TABLE child(parent_id INTEGER NOT NULL REFERENCES parent(id));",
        )
        .await
        .unwrap();
    assert!(
        connection
            .execute("INSERT INTO child(parent_id) VALUES (99)", ())
            .await
            .is_err()
    );

    let transaction = connection.transaction().await.unwrap();
    transaction
        .execute("INSERT INTO parent(id) VALUES (1)", ())
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    let mut rows = connection
        .query("SELECT COUNT(*) FROM parent", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<u32>(0).unwrap(), 0);
    drop(rows);
    drop(connection);
    drop(database);
    fs::remove_file(path).unwrap();
}
