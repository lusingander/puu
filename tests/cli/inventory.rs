use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

use rstest::rstest;

#[rstest]
#[case::render("render", "txt")]
#[case::error("error", "error")]
fn fixture_inputs_and_expectations_are_paired(
    #[case] group: &str,
    #[case] expected_extension: &str,
) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(group);

    assert_eq!(
        stems(&directory, "json"),
        stems(&directory, expected_extension)
    );
}

fn stems(directory: &Path, extension: &str) -> BTreeSet<OsString> {
    fs::read_dir(directory)
        .expect("fixture directory should exist")
        .map(|entry| entry.expect("fixture entry should be readable").path())
        .filter(|path| path.extension() == Some(OsStr::new(extension)))
        .map(|path| {
            path.file_stem()
                .expect("fixture should have a file stem")
                .to_owned()
        })
        .collect()
}
