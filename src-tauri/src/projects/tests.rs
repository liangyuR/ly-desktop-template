use super::*;
use std::io::Read;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[test]
fn failed_switch_preserves_previous_project_after_restart() {
    let data = tempdir().unwrap();
    let hub = Projects::open(data.path()).unwrap();
    let first = hub
        .create("A".into(), "".into())
        .unwrap()
        .active_id
        .unwrap();
    let second = hub
        .create("B".into(), "".into())
        .unwrap()
        .projects
        .into_iter()
        .find(|p| p.id != first)
        .unwrap()
        .id;
    fs::remove_file(hub.dir(&second).unwrap().join("project.json")).unwrap();
    assert!(hub.switch(&second).is_err());
    let reopened = Projects::open(data.path()).unwrap().snapshot().unwrap();
    assert_eq!(reopened.active_id.as_deref(), Some(first.as_str()));
}

#[test]
fn importing_same_package_twice_creates_independent_portable_projects() {
    let data = tempdir().unwrap();
    let backup = tempdir().unwrap();
    let hub = Projects::open(data.path()).unwrap();
    let original = hub
        .create("视觉调试".into(), "样板".into())
        .unwrap()
        .active_id
        .unwrap();
    let source = hub.dir(&original).unwrap();
    fs::create_dir(source.join("portable/业务")).unwrap();
    fs::write(source.join("portable/业务/config.json"), b"original").unwrap();
    fs::write(source.join("machine-private"), b"local setting").unwrap();
    let pack = backup.path().join("project.lyproj");
    hub.export(&original, &pack).unwrap();
    hub.import(&pack).unwrap();
    let final_state = hub.import(&pack).unwrap();
    let imports: Vec<_> = final_state
        .projects
        .iter()
        .filter(|p| p.id != original)
        .collect();
    assert_eq!(imports.len(), 2);
    assert_ne!(imports[0].id, imports[1].id);
    let path_a = hub.dir(&imports[0].id).unwrap();
    let path_b = hub.dir(&imports[1].id).unwrap();
    fs::write(path_a.join("portable/业务/config.json"), b"modified").unwrap();
    assert_eq!(
        fs::read(path_b.join("portable/业务/config.json")).unwrap(),
        b"original"
    );
    assert!(!path_a.join("machine-private").exists());
    assert!(!path_b.join("machine-private").exists());
    assert_eq!(final_state.active_id.as_deref(), Some(original.as_str()));
}

#[test]
fn invalid_package_leaves_existing_project_and_catalog_intact() {
    let data = tempdir().unwrap();
    let backup = tempdir().unwrap();
    let hub = Projects::open(data.path()).unwrap();
    let original = hub
        .create("已有项目".into(), "".into())
        .unwrap()
        .active_id
        .unwrap();
    let path = hub.dir(&original).unwrap().join("portable/data.txt");
    fs::write(&path, b"original").unwrap();
    let pack = backup.path().join("original.lyproj");
    hub.export(&original, &pack).unwrap();
    let mut raw = String::new();
    ZipArchive::new(File::open(pack).unwrap())
        .unwrap()
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut raw)
        .unwrap();
    for (case, rel) in [
        ("checksum", "portable/data.txt"),
        ("path", "../outside.txt"),
    ] {
        let bad = backup.path().join(case);
        let mut w = ZipWriter::new(File::create(&bad).unwrap());
        w.start_file(rel, SimpleFileOptions::default()).unwrap();
        w.write_all(b"modified").unwrap();
        w.start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        w.write_all(raw.replace("portable/data.txt", rel).as_bytes())
            .unwrap();
        w.finish().unwrap();
        assert!(hub.import(&bad).is_err());
        assert_eq!(hub.snapshot().unwrap().projects.len(), 1);
        assert_eq!(
            hub.snapshot().unwrap().active_id.as_deref(),
            Some(original.as_str())
        );
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(&hub.0.root).unwrap().count(), 2);
    }
}
