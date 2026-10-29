"""`fetch`: every file of the shipped record lands only if it hashes to the record."""

import hashlib
import json

import pytest

from dejaview_tools import cli, models


def test_the_shipped_record_names_the_segmenter_and_its_goldens():
    record = models.Record.load()
    assert record.artifact == "eomt-large-ade20k-512.onnx"
    assert len(record.files) == 5
    assert record.release_url(record.artifact).startswith(
        "https://github.com/markhayden/dejaview/releases/download/segmenter-v1/"
    )
    assert record.data["tolerance"] == {"class": 1e-3, "mask": 1e-2, "label_disagreement": 1e-3}


def fake_record(tmp_path, contents):
    source = tmp_path / "source"
    source.mkdir()
    files = {}
    for name, body in contents.items():
        (source / name).write_bytes(body)
        files[name] = hashlib.sha256(body).hexdigest()
    data = {
        "artifact": "model.onnx",
        "sha256": files["model.onnx"],
        "goldens": [{"file": n, "sha256": h} for n, h in files.items() if n != "model.onnx"],
    }
    return source, models.Record("fake-v1", data)


def test_fetch_copies_verified_files_and_keeps_what_is_already_right(tmp_path):
    source, record = fake_record(tmp_path, {"model.onnx": b"weights", "golden-0.npz": b"gold"})
    where = models.fetch(record, tmp_path / "models", str(source))
    assert where == tmp_path / "models" / "fake-v1"
    assert (where / "model.onnx").read_bytes() == b"weights"
    assert (where / "golden-0.npz").read_bytes() == b"gold"
    # A second fetch finds both present and touches nothing.
    (source / "model.onnx").write_bytes(b"changed upstream")
    models.fetch(record, tmp_path / "models", str(source))
    assert (where / "model.onnx").read_bytes() == b"weights"


def test_a_file_that_does_not_hash_to_the_record_is_refused_and_nothing_is_kept(tmp_path):
    source, record = fake_record(tmp_path, {"model.onnx": b"weights"})
    (source / "model.onnx").write_bytes(b"tampered")
    with pytest.raises(models.FetchError, match="is not the recorded"):
        models.fetch(record, tmp_path / "models", str(source))
    assert list((tmp_path / "models" / "fake-v1").iterdir()) == []


def test_the_cli_reports_a_refusal(tmp_path, capsys, monkeypatch):
    source, record = fake_record(tmp_path, {"model.onnx": b"weights"})
    (source / "model.onnx").write_bytes(b"tampered")
    monkeypatch.setattr(models.Record, "load", staticmethod(lambda name="x": record))
    code = cli.main(["fetch", "--from", str(source), "--models", str(tmp_path / "models")])
    assert code == 1
    assert "is not the recorded" in capsys.readouterr().err
    (source / "model.onnx").write_bytes(b"weights")
    assert cli.main(["fetch", "--from", str(source), "--models", str(tmp_path / "models")]) == 0
    assert "complete" in capsys.readouterr().out


def test_the_models_directory_follows_xdg(monkeypatch, tmp_path):
    monkeypatch.setenv("XDG_CACHE_HOME", str(tmp_path))
    assert models.models_dir() == tmp_path / "dejaview" / "models"
    record = models.Record("fake-v1", json.loads('{"artifact": "m.onnx", "sha256": "0"}'))
    assert models.artifact_path(record) == tmp_path / "dejaview" / "models" / "fake-v1" / "m.onnx"


def test_a_source_that_is_neither_a_directory_nor_a_url_is_refused_and_nothing_is_left(tmp_path):
    _, record = fake_record(tmp_path, {"model.onnx": b"weights"})
    with pytest.raises(models.FetchError, match="neither a directory nor an http"):
        models.fetch(record, tmp_path / "models", str(tmp_path / "missing"))
    assert list((tmp_path / "models" / "fake-v1").iterdir()) == []


def test_a_stream_that_breaks_mid_way_leaves_no_temp_file(tmp_path, monkeypatch):
    source, record = fake_record(tmp_path, {"model.onnx": b"weights" * 1000})

    class Breaks:
        def __init__(self):
            self.n = 0

        def read(self, size):
            self.n += 1
            if self.n == 1:
                return b"weig"
            raise OSError("connection reset")

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

    monkeypatch.setattr(models, "_open", lambda source, file: Breaks())
    with pytest.raises(models.FetchError, match="cannot fetch"):
        models.fetch(record, tmp_path / "models", str(source))
    assert list((tmp_path / "models" / "fake-v1").iterdir()) == []


def test_model_md_agrees_with_the_shipped_record():
    from pathlib import Path

    text = (Path(__file__).parent.parent / "MODEL.md").read_text()
    record = models.Record.load()
    assert record.data["sha256"] in text
    for golden in record.data["goldens"]:
        assert golden["sha256"] in text, golden["file"]
    assert f"{record.data['size_bytes']:,}" in text
