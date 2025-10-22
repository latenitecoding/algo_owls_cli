use crate::OWL_DIR;
use crate::common::OwlError;
use crate::owl_utils::cmd_utils;
use crate::owl_utils::fs_utils;
use std::fs;
use std::path::Path;

pub fn show_and_glow(target_path: &Path) -> Result<(), OwlError> {
    cmd_utils::bat_file(target_path).or_else(|_| {
        cmd_utils::glow_file(target_path).or_else(|_| {
            fs::read_to_string(target_path)
                .map(|contents| println!("{}", contents))
                .map_err(|e| {
                    OwlError::FileError(
                        format!("could not show file '{}'", target_path.to_string_lossy()),
                        e.to_string(),
                    )
                })
        })
    })
}

pub fn show_it(target_path: &Path) -> Result<(), OwlError> {
    cmd_utils::bat_file(target_path).or_else(|_| {
        fs::read_to_string(target_path)
            .map(|contents| println!("{}", contents))
            .map_err(|e| {
                OwlError::FileError(
                    format!("could not show file '{}'", target_path.to_string_lossy()),
                    e.to_string(),
                )
            })
    })
}

pub async fn show_quest(
    quest_name: &str,
    case_id: Option<usize>,
    show_ans: bool,
) -> Result<(), OwlError> {
    let quest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !quest_path.exists() {
        super::fetch_quest(quest_name).await?;
    }

    let test_cases = if show_ans {
        fs_utils::find_by_ext(&quest_path, "ans")?
    } else {
        fs_utils::find_by_ext(&quest_path, "in")?
    };

    if let Some(case_number) = case_id {
        let test_case = &test_cases[(case_number + 1) % test_cases.len()];

        show_it(test_case)
    } else {
        for test_case in test_cases {
            show_it(&test_case)?;
        }

        Ok(())
    }
}

pub async fn show_test(quest_name: &str, test_name: &str, show_ans: bool) -> Result<(), OwlError> {
    let quest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !quest_path.exists() {
        super::fetch_quest(quest_name).await?;
    }

    let test_case = if show_ans {
        fs_utils::find_by_stem_and_ext(&quest_path, test_name, "ans")?
    } else {
        fs_utils::find_by_stem_and_ext(&quest_path, test_name, "in")?
    };

    show_it(&test_case)
}
