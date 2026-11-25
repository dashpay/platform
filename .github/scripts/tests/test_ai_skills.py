"""Keep the AI skills and the book page that lists them in step.

Agents find a skill through the `name` and `description` in its SKILL.md, and
people find it through book/src/contributing/ai-skills.md. A skill without
front matter is never offered, and one missing from the page is invisible to
everyone who does not already know it exists.
"""
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[3]
SKILLS = ROOT / ".claude/skills"
PAGE = ROOT / "book/src/contributing/ai-skills.md"


def front_matter(path):
    """The top-level `key: value` pairs of a SKILL.md's front matter."""
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return {}
    block = text[4:].split("\n---", 1)[0]
    return dict(re.findall(r"^([A-Za-z-]+):[ \t]*(.+)$", block, re.M))


def skill_folders():
    return sorted(path.name for path in SKILLS.iterdir() if path.is_dir())


def listed_skills():
    """The skills named in the first column of the page's table."""
    return re.findall(r"^\| `([^`]+)` \|", PAGE.read_text(encoding="utf-8"), re.M)


class AiSkillsTest(unittest.TestCase):
    def test_should_give_every_skill_a_name_and_a_description(self):
        for folder in skill_folders():
            with self.subTest(skill=folder):
                skill = SKILLS / folder / "SKILL.md"
                self.assertTrue(skill.is_file(), f"{folder} has no SKILL.md")
                fields = front_matter(skill)
                self.assertEqual(fields.get("name"), folder)
                self.assertTrue(fields.get("description", "").strip("\"' "))

    def test_should_list_every_skill_in_the_book_once(self):
        listed = listed_skills()
        self.assertEqual(sorted(listed), skill_folders())
        self.assertEqual(listed, sorted(listed), "keep the table in alphabetical order")

    def test_should_link_the_page_from_the_book_summary(self):
        summary = (ROOT / "book/src/SUMMARY.md").read_text(encoding="utf-8")
        self.assertIn("(contributing/ai-skills.md)", summary)

    def test_should_share_the_skills_with_codex(self):
        self.assertEqual((ROOT / ".codex/skills").resolve(), SKILLS.resolve())


if __name__ == "__main__":
    unittest.main()
