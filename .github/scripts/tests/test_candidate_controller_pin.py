"""Keep the trusted candidate caller on the reviewed exact-contract controller."""
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[3]
REVIEWED_CONTROLLER = "1be03edb7f4f18548d525b17dfac6ee31db17c99"


class CandidateControllerPinTests(unittest.TestCase):
    def test_should_bind_workflow_and_checkout_to_the_same_reviewed_commit(self):
        """Both pins must adopt the reviewed merge, never mutable or mixed refs."""
        workflow = (ROOT / ".github/workflows/runner-image-candidate.yml").read_text()
        workflow_refs = re.findall(
            r"^\s+uses: dashpay/dash-selfhosted-image/"
            r"\.github/workflows/platform-candidate\.yml@(\S+)\s*$",
            workflow, re.MULTILINE,
        )
        checkout_refs = re.findall(
            r"^\s+control_revision: (\S+)\s*$", workflow, re.MULTILINE,
        )
        self.assertEqual(workflow_refs, [REVIEWED_CONTROLLER])
        self.assertEqual(checkout_refs, [REVIEWED_CONTROLLER])
