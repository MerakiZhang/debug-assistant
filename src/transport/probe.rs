use probe_rs::probe::list::Lister;

pub fn list_probes() -> Vec<String> {
    let probes = Lister::new().list_all();
    if probes.is_empty() {
        return vec!["(no probes found)".to_string()];
    }

    probes
        .iter()
        .enumerate()
        .map(|(idx, probe)| format!("{}: {}", idx, probe))
        .collect()
}
