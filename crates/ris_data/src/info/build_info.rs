#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct BuildInfo {
    git_repo: String,
    git_commit: String,
    git_branch: String,
    rustc_version: String,
    rustup_toolchain: String,
    build_profile: String,
    build_date: String,
}

impl BuildInfo {
    pub fn new() -> BuildInfo {
        //@@AUTO GENERATE START@@
        BuildInfo {
            git_repo: String::from(r"https://github.com/Rismosch/ris_engine.git"),
            git_commit: String::from(r"732da61e846cde06794559083be0cbd55a4718c7"),
            git_branch: String::from(r"dev"),
            rustc_version: String::from(r"rustc 1.75.0 (82e1608df 2023-12-21)"),
            rustup_toolchain: String::from(
                r"stable-x86_64-unknown-linux-gnu (directory override for /home/simon/repos/ris_engine)",
            ),
            build_profile: profile(),
            build_date: String::from(r"2024-02-02 19:43:41.165877308+01:00"),
        }
        //@@AUTO GENERATE END@@
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(debug_assertions)]
fn profile() -> String {
    String::from("debug")
}

#[cfg(not(debug_assertions))]
fn profile() -> String {
    String::from("release")
}

impl std::fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Build")?;
        writeln!(f, "git repo:            {}", self.git_repo)?;
        writeln!(f, "git commit:          {}", self.git_commit)?;
        writeln!(f, "git branch:          {}", self.git_branch)?;
        writeln!(f, "compiler:            {}", self.rustc_version)?;
        writeln!(f, "toolchain:           {}", self.rustup_toolchain)?;
        writeln!(f, "profile:             {}", self.build_profile)?;
        writeln!(f, "build date:          {}", self.build_date)?;

        Ok(())
    }
}
