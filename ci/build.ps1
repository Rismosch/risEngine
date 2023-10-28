$purpose = "This script generates build info and compiles the workspace as a release ready package."

$ErrorActionPreference = "Stop"
Import-Module "$PSScriptRoot/util.ps1" -force
Push-Location $root_dir

try {
    Write-Host "checking preconditions..."

    $sdl2_dll_path = "$root_dir/SDL2.dll"
    $sdl2_dll_exists = Test-Path $sdl2_dll_path

    if (!$sdl2_dll_exists) {
        throw "could not find ``SDL2.dll`` in the root directory"
    }

    Write-Host "clearing destination directory..."

    $final_directory = GetAndClearCiOutDir

    Write-Host "parsing cli args..."

    $cli_default = "--default"

    $cli_cargo_clean = "--cargo-clean"
    $cli_no_cargo_clean = "--no-cargo-clean"
    $cli_cargo_clean_value = $false

    if ($args.length -eq 0) {
        Write-Host ""
        Write-Host $purpose
        Write-Host ""
        Write-Host "INFO: you may skip user input, by providing cli args."
        Write-Host ""
        Write-Host "available args:"
        Write-Host "    $cli_default         skips user input and uses default values for everything below"
        Write-Host ""
        Write-Host "    $cli_cargo_clean     executes ``cargo clean`` before building"
        Write-Host "    $cli_no_cargo_clean  does not execute ``cargo clean`` (default)"
        Write-Host ""
        Write-Host ""
        Write-Host ""
        Write-Host ""
        Write-Host ""

        $user_input = Read-Host "should ``cargo clean`` be executed before building? (y/N)"
        if ($user_input.ToLower() -eq "y") {
            $cli_cargo_clean_value = $true
        }
    } else {
        for($i = 0; $i -lt $args.length; ++$i) {
            $arg = $args[$i]
            switch ($arg) {
                $cli_default { break }
                $cli_cargo_clean { $cli_cargo_clean_value = $true }
                $cli_no_cargo_clean { $cli_cargo_clean_value = $false }
                default { throw "unkown cli arg: $arg" }
            }
        }
    }

    Write-Host "generating build info..."

    $build_info_path = "$PSScriptRoot/../crates/ris_data/src/info/build_info.rs"

    function RunCommand {
        param (
            $command
        )

        try {
            Write-Host "running command: $command"
            return Invoke-Expression $command
        }
        catch {
            return "error while running ``$command``"
        }
    }

    $git_repo = RunCommand "git config --get remote.origin.url"
    $git_commit = RunCommand "git rev-parse HEAD"
    $git_branch = RunCommand "git rev-parse --abbrev-ref HEAD"

    $rustc_version = RunCommand "rustc --version"
    $rustup_toolchain = RunCommand "rustup show active-toolchain"

    $build_date = Get-Date -Format "o"

    $build_info_content =
    "// DO NOT COMMIT CHANGES TO THIS FILE.
    // DO NOT MODIFY THIS FILE.
    //
    // THE CONTENTS OF THIS FILE ARE AUTOMATICALLY GENERATED BY THE BUILD SCRIPT.
    //
    // I highly recommend you run the following git command:
    // git update-index --assume-unchanged crates/ris_data/src/info/build_info.rs
    //
    // Doc: https://git-scm.com/docs/git-update-index#_using_assume_unchanged_bit

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
            BuildInfo {
                git_repo: String::from(r`"$git_repo`"),
                git_commit: String::from(r`"$git_commit`"),
                git_branch: String::from(r`"$git_branch`"),
                rustc_version: String::from(r`"$rustc_version`"),
                rustup_toolchain: String::from(r`"$rustup_toolchain`"),
                build_profile: profile(),
                build_date: String::from(r`"$build_date`"),
            }
        }
    }

    impl Default for BuildInfo {
        fn default() -> Self {
            Self::new()
        }
    }

#[cfg(debug_assertions)]
    fn profile() -> String {
        String::from(`"debug`")
    }

#[cfg(not(debug_assertions))]
    fn profile() -> String {
        String::from(`"release`")
    }

    impl std::fmt::Display for BuildInfo {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            writeln!(f, `"Build`")?;
            writeln!(f, `"git repo:            {}`", self.git_repo)?;
            writeln!(f, `"git commit:          {}`", self.git_commit)?;
            writeln!(f, `"git branch:          {}`", self.git_branch)?;
            writeln!(f, `"compiler:            {}`", self.rustc_version)?;
            writeln!(f, `"toolchain:           {}`", self.rustup_toolchain)?;
            writeln!(f, `"profile:             {}`", self.build_profile)?;
            writeln!(f, `"build date:          {}`", self.build_date)?;

            Ok(())
        }
    }
    "

    Set-Content -Path $build_info_path -Value $build_info_content


    Write-Host "cleaning workspace..."
    if ($cli_cargo_clean_value -eq $true) {
        cargo clean
    }

    Write-Host "importing assets..."
    cargo run -p ris_asset_compiler importall
    Write-Host "compiling assets..."
    cargo run -p ris_asset_compiler compile

    Write-Host "compiling workspace..."
    cargo build -r

    Write-Host "moving files..."

    $target_directory = Resolve-Path "$root_dir/target/release"
    $source_exe_path = Resolve-Path "$target_directory/ris_engine.exe"
    $asset_filename = "ris_assets"
    $asset_path = Resolve-Path "$root_dir/$asset_filename"

    Copy-Item $source_exe_path -Destination "$final_directory/ris_engine.exe"
    Copy-Item $sdl2_dll_path -Destination "$final_directory/SDL2.dll"
    Copy-Item $asset_path -Destination "$final_directory/$asset_filename"

    Write-Host "done! final build can be found under ``$final_directory``"

}
finally {
    Pop-Location
}
