use crate::OWL_DIR;
use crate::common::{OwlError, Result};
use crate::owl_utils::{FileApp, FileExplorerApp, cmd_utils, fs_utils, tui_utils};
use std::fs;
use std::path::Path;

pub fn show_and_glow(target_path: &Path) -> Result<()> {
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

pub fn show_it(target_path: &Path) -> Result<()> {
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
    use_tui: bool,
) -> Result<()> {
    let quest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !quest_path.exists() {
        super::fetch_quest(quest_name).await?;
    }

    if use_tui && case_id.is_none() {
        return tui_utils::enter_raw_mode().and_then(|_| {
            match FileExplorerApp::default().run(&quest_path) {
                Ok(_) => tui_utils::exit_raw_mode(),
                Err(e) => tui_utils::exit_raw_mode().and(Err(e)),
            }
        });
    }

    let test_cases = if show_ans {
        fs_utils::find_by_ext(&quest_path, "ans")?
    } else {
        fs_utils::find_by_ext(&quest_path, "in")?
    };

    if let Some(case_number) = case_id {
        let test_case = &test_cases[(case_number - 1) % test_cases.len()];

        if use_tui {
            tui_utils::enter_raw_mode().and_then(|_| match FileApp::default().run(test_case) {
                Ok(_) => tui_utils::exit_raw_mode(),
                Err(e) => tui_utils::exit_raw_mode().and(Err(e)),
            })
        } else {
            show_it(test_case)
        }
    } else {
        for test_case in test_cases {
            show_it(&test_case)?;
        }

        Ok(())
    }
}

pub async fn show_test(
    quest_name: &str,
    test_name: &str,
    show_ans: bool,
    use_tui: bool,
) -> Result<()> {
    let quest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !quest_path.exists() {
        super::fetch_quest(quest_name).await?;
    }

    let test_case = if show_ans {
        fs_utils::find_by_stem_and_ext(&quest_path, test_name, "ans")?
    } else {
        fs_utils::find_by_stem_and_ext(&quest_path, test_name, "in")?
    };

    if use_tui {
        tui_utils::enter_raw_mode().and_then(|_| match FileApp::default().run(&test_case) {
            Ok(_) => tui_utils::exit_raw_mode(),
            Err(e) => tui_utils::exit_raw_mode().and(Err(e)),
        })
    } else {
        show_it(&test_case)
    }
}
