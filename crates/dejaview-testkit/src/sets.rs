//! A synthetic source-fidelity evaluation set for tests: `families` families of three items each,
//! two expected fails (the rotated fixture pair) and one expected pass (the gradient pair), with
//! labels beside the manifest. Family ids are `<prefix>-<n>`, item ids `<family>-a|b|c`. The
//! caller seals it (`dejaview sets seal`), moving the labels out for a blind set.

use std::path::{Path, PathBuf};

use dejaview_core::image::sha256_hex;

/// Writes the set under `evalsets_root/<id>` from the fixture pairs at `pairs_dir`
/// (`fixtures/evalsets/source-fidelity/pairs-v1/items`) and returns the set directory.
pub fn synthetic_set(
    evalsets_root: &Path,
    pairs_dir: &Path,
    id: &str,
    split: &str,
    prefix: &str,
    families: usize,
) -> crate::Result<PathBuf> {
    let io = |e: std::io::Error| crate::Error::Script(e.to_string());
    let dir = evalsets_root.join(id);
    std::fs::create_dir_all(&dir).map_err(io)?;
    let mut manifest = format!(
        "[set]\nid = {id:?}\nuse_case = \"source-fidelity\"\nsplit = {split:?}\nlicence = \"Apache-2.0\"\nimages_repo = \".\"\nlabels = \"labels.toml\"\nfamilies = \"families.toml\"\nrules = {{ base = \"source-fidelity\", version = 1 }}\n"
    );
    let mut fams = String::new();
    let mut labels = String::new();
    for f in 0..families {
        let family = format!("{prefix}-{f}");
        fams.push_str(&format!(
            "[[family]]\nid = {family:?}\ndescription = \"synthetic family {f}\"\n\n"
        ));
        for (suffix, pair, expected) in [
            ("a", "pair-02", "fail"),
            ("b", "pair-02", "fail"),
            ("c", "pair-01", "pass"),
        ] {
            let item = format!("{family}-{suffix}");
            let item_dir = dir.join("items").join(&item);
            std::fs::create_dir_all(&item_dir).map_err(io)?;
            let mut job = String::from(
                "[job]\nuse_case = \"source-fidelity\"\nbrief = \"Restyle the scene; keep the structure.\"\n",
            );
            for (image, role) in [("candidate", "candidate"), ("source", "source")] {
                let bytes =
                    std::fs::read(pairs_dir.join(pair).join(format!("{image}.png"))).map_err(io)?;
                std::fs::write(item_dir.join(format!("{image}.png")), &bytes).map_err(io)?;
                job.push_str(&format!(
                    "\n[[job.images]]\nid = {image:?}\nrole = {role:?}\nsource = {{ path = \"items/{item}/{image}.png\", sha256 = {:?} }}\n",
                    sha256_hex(&bytes)
                ));
            }
            std::fs::write(item_dir.join("job.toml"), job).map_err(io)?;
            manifest.push_str(&format!(
                "\n[[item]]\nid = {item:?}\nfamily = {family:?}\njob = \"items/{item}/job.toml\"\n"
            ));
            let (types, ids) = if expected == "fail" {
                ("[\"rotated\"]", "[\"SF2\"]")
            } else {
                ("[]", "[]")
            };
            labels.push_str(&format!(
                "[[label]]\nitem = {item:?}\nexpected = {expected:?}\nfailure_types = {types}\nwords = \"synthetic\"\nobligation_ids = {ids}\nadjudicated_by = \"test\"\nadjudicated_on = \"2026-10-27\"\n"
            ));
            if expected == "fail" {
                labels.push_str(
                    "\n  [[label.regions]]\n  frame = \"raw:candidate\"\n  kind = \"whole_image\"\n  defect = \"rotated\"\n  obligation_ids = [\"SF2\"]\n",
                );
            }
            labels.push('\n');
        }
    }
    std::fs::write(dir.join("set.toml"), manifest).map_err(io)?;
    std::fs::write(dir.join("families.toml"), fams).map_err(io)?;
    std::fs::write(dir.join("labels.toml"), labels).map_err(io)?;
    Ok(dir)
}
