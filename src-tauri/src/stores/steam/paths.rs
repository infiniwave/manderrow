use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, ensure, Result};
use manderrow_paths::home_dir;
use slog::warn;

#[cfg(windows)]
pub fn get_steam_install_path_from_registry() -> Result<PathBuf> {
    use registry::{Data, Hive, Security};
    let regkey = Hive::LocalMachine.open(r"SOFTWARE\\WOW6432Node\\Valve\\Steam", Security::Read)?;
    match regkey.value("InstallPath")? {
        Data::String(s) | Data::ExpandString(s) => Ok(PathBuf::from(s.to_string()?)),
        _ => Err(anyhow!("Unexpected data type in registry")),
    }
}

pub fn get_steam_exe() -> Result<Cow<'static, Path>> {
    if cfg!(windows) {
        #[cfg(windows)]
        {
            let mut p = get_steam_install_path_from_registry()?;
            p.push("steam.exe");
            Ok(Cow::Owned(p))
        }
        #[cfg(not(windows))]
        unreachable!()
    } else if cfg!(target_os = "macos") {
        Ok(Cow::Borrowed(Path::new(
            "/Applications/Steam.app/Contents/MacOS/steam_osx",
        )))
    } else if cfg!(unix) {
        Ok(Cow::Borrowed(Path::new("steam")))
    } else {
        return Err(anyhow!("Unsupported platform for Steam").into());
    }
}

pub async fn resolve_steam_directory() -> Result<PathBuf> {
    const ERROR_MSG: &str = "Could not locate Steam";
    if cfg!(target_os = "macos") {
        let path = home_dir().join("Library/Application Support/Steam");
        if tokio::fs::try_exists(&path).await? {
            Ok(path)
        } else {
            Err(anyhow::Error::msg(ERROR_MSG))
        }
    } else if cfg!(target_os = "linux") {
        const PREFIXES: &[&[&str]] = &[&[], &[".var", "app", "com.valvesoftware.Steam"]];
        const PATHS: &[&[&str]] = &[
            &[".local", "share", "Steam"],
            &[".steam", "steam"],
            &[".steam", "root"],
            &[".steam"],
        ];
        let mut buf = home_dir().to_owned();
        for &prefix in PREFIXES {
            for &segment in prefix {
                buf.push(segment);
            }
            for &path in PATHS {
                for &segment in path {
                    buf.push(segment);
                }
                if tokio::fs::try_exists(&buf).await? {
                    return Ok(buf);
                }
                for _ in path {
                    buf.pop();
                }
            }
            for _ in prefix {
                buf.pop();
            }
        }
        Err(anyhow::Error::msg(ERROR_MSG))
    } else if cfg!(windows) {
        #[cfg(windows)]
        {
            get_steam_install_path_from_registry()
        }
        #[cfg(not(windows))]
        unreachable!()
    } else {
        Err(anyhow!("Unsupported operating system"))
    }
}

pub async fn resolve_steamapps_directory() -> Result<PathBuf> {
    const ERROR_MSG: &str = "Could not locate steamapps directory";
    let mut buf = resolve_steam_directory().await?;
    // these are the subpaths searched by r2modman
    const PATHS: &[&[&str]] = &[
        &["steamapps"], // this one is the most standard path
        &["steam", "steamapps"],
        &["root", "steamapps"],
    ];
    for &path in PATHS {
        for &segment in path {
            buf.push(segment);
        }
        if tokio::fs::try_exists(&buf).await? {
            return Ok(buf);
        }
        for _ in path {
            buf.pop();
        }
    }
    Err(anyhow::Error::msg(ERROR_MSG))
}

pub async fn resolve_steam_library_folders() -> Result<Vec<PathBuf>> {
    let steamapps_dir = resolve_steamapps_directory().await?;
    let mut locations = vec![steamapps_dir.clone()];

    let mut iter = tokio::fs::read_dir(&steamapps_dir).await?;
    let mut path_buf = steamapps_dir;
    while let Some(e) = iter.next_entry().await? {
        let name = e.file_name();
        if name.eq_ignore_ascii_case("libraryfolders.vdf") {
            path_buf.push(name);
            tokio::task::block_in_place(|| {
                let mut rdr = vdf::Reader::new(std::fs::File::open(&path_buf)?);
                let Some(vdf::Event::GroupStart { key, .. }) = rdr.next()? else {
                    bail!("Invalid libraryfolders.vdf file: Invalid VDF file")
                };
                if !key.s.eq_ignore_ascii_case(b"libraryfolders") {
                    bail!("Invalid libraryfolders.vdf file: Unexpected root key")
                }
                while let Some(event) = rdr.next()? {
                    match event {
                        vdf::Event::GroupEnd { .. } => break,
                        vdf::Event::GroupStart { .. } => {
                            let mut depth = 0;
                            while let Some(event) = rdr.next()? {
                                match event {
                                    vdf::Event::GroupStart { .. } => depth += 1,
                                    vdf::Event::GroupEnd { .. } if depth == 0 => break,
                                    vdf::Event::GroupEnd { .. } => depth -= 1,
                                    vdf::Event::Item { key, value, .. } if key.s == b"path" => {
                                        locations.push(value.validate_utf8()?.s.into());
                                    }
                                    vdf::Event::Item { .. } => {}
                                    vdf::Event::Comment { .. } => {}
                                    vdf::Event::FileEnd { .. } => bail!("Unexpected EOF"),
                                }
                            }
                        }
                        vdf::Event::Item { value, .. } => {
                            locations.push(value.validate_utf8()?.s.into());
                        }
                        vdf::Event::Comment { .. } => {}
                        vdf::Event::FileEnd { .. } => {}
                    }
                }
                Result::Ok(())
            })?;
            path_buf.pop();
        }
    }
    Ok(locations)
}

/// The `game_id` is Steam's numerical id for the game.
pub async fn resolve_steam_app_manifest(log: &slog::Logger, game_id: &str) -> Result<PathBuf> {
    let target_name = format!("appmanifest_{game_id}.acf");

    let library_folders = resolve_steam_library_folders().await?;
    for (i, path) in library_folders.iter().enumerate() {
        let mut iter = match tokio::fs::read_dir(&path).await {
            Ok(t) => t,
            Err(e) => {
                warn!(
                    log,
                    "Failed to read steam library folder at {:?}: {}", path, e
                );
                continue;
            }
        };
        while let Some(e) = iter.next_entry().await? {
            let name = e.file_name();
            if name.eq_ignore_ascii_case(&target_name) {
                let mut path = library_folders.into_iter().nth(i).unwrap();
                path.push(name);
                return Ok(path);
            }
        }
    }
    Err(anyhow!(
        "Unable to locate game app manifest for {game_id:?} in {library_folders:?}"
    ))
}

/// The `game_id` is Steam's numerical id for the game.
pub async fn resolve_steam_app_compat_data_directory(
    log: &slog::Logger,
    game_id: &str,
) -> Result<PathBuf> {
    let mut path = resolve_steam_app_manifest(log, game_id).await?;
    ensure!(path.pop(), "This should not be");
    path.push("compatdata");
    path.push(game_id);
    Ok(path)
}

/// The `game_id` is Steam's numerical id for the game.
pub async fn resolve_app_install_directory(log: &slog::Logger, game_id: &str) -> Result<PathBuf> {
    let manifest = resolve_steam_app_manifest(log, game_id).await?;
    tokio::task::block_in_place(|| {
        let mut rdr = vdf::Reader::new(std::fs::File::open(&manifest)?);
        let Some(vdf::Event::GroupStart { key, .. }) = rdr.next()? else {
            bail!("Invalid app manifest file: Invalid VDF file")
        };
        if !key.s.eq_ignore_ascii_case(b"AppState") {
            bail!("Invalid app manifest file: Unexpected root key")
        }
        while let Some(event) = rdr.next()? {
            match event {
                vdf::Event::GroupEnd { .. } => break,
                vdf::Event::GroupStart { .. } => {
                    let mut depth = 0;
                    while let Some(event) = rdr.next()? {
                        match event {
                            vdf::Event::GroupStart { .. } => depth += 1,
                            vdf::Event::GroupEnd { .. } if depth == 0 => break,
                            vdf::Event::GroupEnd { .. } => depth -= 1,
                            vdf::Event::Item { .. } => {}
                            vdf::Event::Comment { .. } => {}
                            vdf::Event::FileEnd { .. } => bail!("Unexpected EOF"),
                        }
                    }
                }
                vdf::Event::Item { key, value, .. } if key.s == b"installdir" => {
                    let mut path = manifest;
                    ensure!(path.pop(), "This should not be");
                    path.push("common");
                    path.push(value.validate_utf8()?.s);
                    return Ok(path);
                }
                vdf::Event::Item { .. } => {}
                vdf::Event::Comment { .. } => {}
                vdf::Event::FileEnd { .. } => {}
            }
        }
        Err(anyhow!(
            "Unable to determine install path for game {game_id:?}"
        ))
    })
}
