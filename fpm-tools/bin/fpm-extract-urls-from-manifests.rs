use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::path;

use lazy_static::lazy_static;

use fpm::flatpak_manifest::{
    FlatpakManifest, FlatpakModule, FlatpakModuleDescription, FlatpakSource, FlatpakSourceDescription,
};

fn main() {
    fpm::logger::init();
    let db = fpm::db::Database::get_database();

    if db.indexed_projects.len() == 0 {
        panic!("There are no projects in the database!");
    }

    let mut all_git_urls_from_manifests: HashSet<String> = HashSet::new();
    let mut all_archive_urls: HashSet<String> = HashSet::new();

    for (project_id, project) in &db.indexed_projects {
        // We're only interested in having stats for the projects supporting Flatpak.
        if !project.supports_flatpak() {
            continue;
        }

        log::info!("Processing project {}...", project_id);
        let repo_url = project.get_main_vcs_url();

        let repo_dir = match fpm::utils::clone_git_repo(&repo_url) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Could not clone repo {}: {}", &repo_url, e);
                continue;
            }
        };

        for manifest_path in &project.flatpak_app_manifests {
            let absolute_manifest_path = repo_dir.to_string() + manifest_path;

            let flatpak_manifest = match FlatpakManifest::load_from_file(absolute_manifest_path.to_string()) {
                Some(m) => m,
                None => {
                    log::warn!(
                        "Could not parse Flatpak manifest at {}!!!",
                        absolute_manifest_path
                    );
                    continue;
                }
            };

            // FIXME we should get the modules recursively here!!!
            for module in &flatpak_manifest.modules {
                let module_description = match &module {
                    FlatpakModule::Path(_) => continue,
                    FlatpakModule::Description(d) => d,
                };

                for git_url in module_description.get_all_git_urls() {
                    all_git_urls_from_manifests.insert(git_url.to_string());
                }
                for archive_url in module_description.get_all_archive_urls() {
                    // FIXME remove this after testing!!!
                    if all_archive_urls.len() > 20 {
                        continue;
                    }
                    all_archive_urls.insert(archive_url.to_string());
                }
            }
        }

        for manifest_path in &project.flatpak_module_manifests {
            let absolute_manifest_path = repo_dir.to_string() + manifest_path;
            let module_description =
                FlatpakModuleDescription::load_from_file(absolute_manifest_path).unwrap();
            for git_url in module_description.get_all_git_urls() {
                all_git_urls_from_manifests.insert(git_url.to_string());
            }
            for archive_url in module_description.get_all_archive_urls() {
                all_archive_urls.insert(archive_url.to_string());
            }
        }
    }

    log::info!(
        "Extracted {} git urls from the manifests",
        all_git_urls_from_manifests.len()
    );
    log::info!(
        "Extracted {} archive urls from the manifests",
        all_archive_urls.len()
    );

    let mut git_urls_dump = "".to_string();
    for git_url in &all_git_urls_from_manifests {
        git_urls_dump += &format!("{}\n", git_url);
    }
    let git_urls_dump_path = format!("{}/git_urls_from_manifests.txt", fpm::db::Database::get_db_path());
    let git_urls_dump_path = path::Path::new(&git_urls_dump_path);
    if let Err(e) = fs::write(git_urls_dump_path, &git_urls_dump) {
        log::warn!(
            "Could not save the dump for git URLs to {}: {}.",
            git_urls_dump_path.display(),
            e
        );
    };

    let mut archive_urls_dump = "".to_string();
    for archive_url in &all_archive_urls {
        archive_urls_dump += &format!("{}\n", archive_url);
    }
    let archive_urls_dump_path = format!(
        "{}/archive_urls_from_manifests.txt",
        fpm::db::Database::get_db_path()
    );
    let archive_urls_dump_path = path::Path::new(&archive_urls_dump_path);
    if let Err(e) = fs::write(archive_urls_dump_path, &archive_urls_dump) {
        log::warn!(
            "Could not save the dump for archive URLS {}: {}.",
            archive_urls_dump_path.display(),
            e
        );
    };
}
