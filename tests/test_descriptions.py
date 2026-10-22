"""Tests for module descriptions and online activity link shortcuts."""

from __future__ import annotations

from pathlib import Path

from ufora_sync.sync import (
    SyncConfig,
    SyncManifest,
    SyncResult,
    _generate_html_shortcut,
    _resolve_topic_url,
    html_to_markdown,
    sync_course_descriptions_and_links,
)


def test_html_to_markdown_basic():
    raw = "<p>Welcome to <strong>Physics</strong>!</p><p>Check the <a href='https://example.com'>syllabus</a>.</p>"
    md = html_to_markdown(raw)
    assert "**Physics**" in md
    assert "[syllabus](https://example.com)" in md


def test_html_to_markdown_unescapes_entities():
    raw = "<p>Onder &#176;Voorbereiding vind je taken v&#243;&#243;r 23u.</p>"
    md = html_to_markdown(raw)
    assert "°Voorbereiding" in md
    assert "vóór" in md


def test_html_to_markdown_empty():
    assert html_to_markdown("") == ""
    assert html_to_markdown(None) == ""


def test_resolve_topic_url():
    # Relative D2L link
    t1 = {"Url": "/d2l/common/dialogs/quickLink/test.d2l", "TopicId": "123"}
    assert (
        _resolve_topic_url("100", t1)
        == "https://ufora.ugent.be/d2l/common/dialogs/quickLink/test.d2l"
    )

    # External URL
    t2 = {"Url": "https://youtu.be/sample", "TopicId": "456"}
    assert _resolve_topic_url("100", t2) == "https://youtu.be/sample"

    # Fallback to direct view
    t3 = {"Url": "", "TopicId": "789"}
    assert (
        _resolve_topic_url("100", t3)
        == "https://ufora.ugent.be/d2l/le/content/100/viewContent/789/View"
    )


def test_generate_html_shortcut():
    url = "https://ufora.ugent.be/quiz/1"
    html_code = _generate_html_shortcut("Test Quiz", url)
    assert f'<meta http-equiv="refresh" content="0; url={url}">' in html_code
    assert "Test Quiz" in html_code
    assert f'window.location.href = "{url}"' in html_code


def test_sync_course_descriptions_and_links(tmp_path: Path):
    course_dir = tmp_path / "Math"
    course_dir.mkdir()
    manifest = SyncManifest(base_dir=course_dir)
    manifest.load()

    toc = {
        "Modules": [
            {
                "ModuleId": 10,
                "Title": "Lab 1",
                "Description": {
                    "Html": "<p>Please prepare <strong>exercise 1</strong>.</p>",
                    "Text": "Please prepare exercise 1.",
                },
                "Topics": [
                    {
                        "TopicId": 201,
                        "Title": "Submit Lab 1",
                        "TypeIdentifier": "Link",
                        "Url": "https://ufora.ugent.be/dropbox/1",
                        "Description": {"Text": "Submit before Friday."},
                    }
                ],
                "Modules": [],
            }
        ]
    }

    cfg = SyncConfig(sync_descriptions=True, sync_links=True)
    res = SyncResult()

    dirty = sync_course_descriptions_and_links("100", course_dir, toc, manifest, cfg, res)
    assert dirty is True
    assert len(res.downloaded) == 2  # README.md and Submit Lab 1.html

    lab_dir = course_dir / "Lab 1"
    readme_file = lab_dir / "README.md"
    shortcut_file = lab_dir / "Submit Lab 1.html"

    assert readme_file.exists()
    assert shortcut_file.exists()

    readme_content = readme_file.read_text(encoding="utf-8")
    assert "# Lab 1" in readme_content
    assert "**exercise 1**" in readme_content
    assert "[Submit Lab 1](https://ufora.ugent.be/dropbox/1)" in readme_content
    assert "Submit before Friday." in readme_content

    # Second pass: should skip both because content didn't change
    res2 = SyncResult()
    dirty2 = sync_course_descriptions_and_links("100", course_dir, toc, manifest, cfg, res2)
    assert dirty2 is False
    assert len(res2.downloaded) == 0
    assert len(res2.skipped_exists) == 2
