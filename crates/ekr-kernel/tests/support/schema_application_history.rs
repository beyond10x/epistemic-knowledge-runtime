// Shared real-provider application retention and reconstruction helpers.

fn captured_application_history(
    path: &std::path::Path,
    sqlite: bool,
    human: &Human,
) -> (ekr_kernel::KernelAuthority, ekr_store::RetainedHistory) {
    use ekr_store::RevisionLog;
    let mut captured = None;
    if sqlite {
        let _kernel = ekr_kernel::Commit::over_with_review_authority(
            context(),
            anchor(),
            human.binding.clone(),
            |authority| {
                let store = ekr_store::SqliteStore::sqlite(path, "upgrade-fixture", None)?
                    .under(authority.clone());
                captured = Some((authority, store.history()?));
                Ok(store)
            },
        )
        .unwrap();
    } else {
        let _kernel = ekr_kernel::Commit::over_with_review_authority(
            context(),
            anchor(),
            human.binding.clone(),
            |authority| {
                let store = ekr_store::FileStore::file(path, "upgrade-fixture", None)?
                    .under(authority.clone());
                captured = Some((authority, store.history()?));
                Ok(store)
            },
        )
        .unwrap();
    }
    captured.unwrap()
}

fn copy_closed_provider(from: &std::path::Path, to: &std::path::Path) {
    if from.is_dir() {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            copy_closed_provider(&entry.path(), &to.join(entry.file_name()));
        }
    } else {
        std::fs::copy(from, to).unwrap();
    }
}

fn application_retention(
    path: &std::path::Path,
    sqlite: bool,
    human: &Human,
) -> Box<dyn ekr_store::ApplicationRetention> {
    let (authority, _) = captured_application_history(path, sqlite, human);
    if sqlite {
        Box::new(
            ekr_store::SqliteStore::sqlite(path, "upgrade-fixture", None)
                .unwrap()
                .under(authority),
        )
    } else {
        Box::new(
            ekr_store::FileStore::file(path, "upgrade-fixture", None)
                .unwrap()
                .under(authority),
        )
    }
}

fn commit_application_document(
    store: &Runtime,
    transaction: TransactionId,
    document: &[u8],
    at: i64,
) {
    store
        .propose(document, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let revision = store.read(None).unwrap().root.revision;
    assert!(matches!(
        store
            .validate(transaction, revision, || Timestamp::from_millis(at))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        store
            .commit(transaction, context().operator, || Timestamp::from_millis(
                at
            ))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
}
