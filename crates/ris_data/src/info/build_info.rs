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
            git_commit: String::from(r"49b782b05ae62e5128f1d777a8fe716740ede6f0"),
            git_branch: String::from(r"main"),
            rustc_version: String::from(r"rustc 1.71.1 (eb26296b5 2023-08-03)"),
            rustup_toolchain: String::from(
                r"stable-x86_64-unknown-linux-gnu (directory override for /home/simon/repos/ris_engine)",
            ),
            build_profile: profile(),
            build_date: String::from(r"2023-11-30 23:27:58.176413040+01:00"),
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
