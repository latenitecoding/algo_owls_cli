use crate::common::{OwlError, Result};
use crate::owl_utils::{cmd_utils, prog_utils};
use std::path::Path;

pub fn run_program(prog: &Path) -> Result<()> {
    if !prog.exists() {
        return Err(OwlError::FileError(
            format!("'{}': program not found", prog.to_string_lossy()),
            "".into(),
        ));
    }

    match prog_utils::check_prog_lang(prog) {
        Some(lang) => {
            let (target, build_files) = match prog_utils::build_program(prog)? {
                Some(bl) => (bl.target, bl.build_files),
                None => (prog.to_path_buf(), None),
            };

            let run_result = lang.run(&target);

            prog_utils::cleanup_program(prog, &target, build_files)?;

            run_result.map(|(stdout, _)| println!("{}", stdout))
        }
        None => {
            let (stdout, _) = cmd_utils::run_binary(prog)?;
            println!("{}", stdout);
            Ok(())
        }
    }
}
