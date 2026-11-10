import copy
import unittest
from deploy.upgrade_gateway import check_current, STORE
from tools.gateway_profile import profile


class UpgradeTests(unittest.TestCase):
    def test_refuses_unexpected_image_scope_mount_and_environment(self):
        image = "sha256:" + "a" * 64
        candidate = profile(image, store=STORE)
        check_current(candidate, image)
        with self.assertRaises(ValueError):
            check_current(profile(image,store='/srv/another-user/snapshots'),image)
        for field, value in [("volumes", ["/:/data:rw"]), ("env", []), ("image", "other")]:
            altered = copy.deepcopy(candidate)
            altered["servers"][0]["snapshot"]["server"][field] = value
            with self.assertRaises(ValueError):
                check_current(altered, image)
        with self.assertRaises(ValueError):
            check_current(candidate, "sha256:" + "b" * 64)
        candidate["id"] = "unrelated"
        with self.assertRaises(ValueError):
            check_current(candidate, image)
