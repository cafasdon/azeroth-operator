import unittest
from shared.clients import ClientFamily, ClientIdentity, require_same_client


class ClientBoundaryTests(unittest.TestCase):
    def identity(self, family=ClientFamily.RETAIL, variant="live", build="123", interface=120105):
        return ClientIdentity(family, build, interface, variant)

    def test_matching_client(self):
        require_same_client(self.identity(), self.identity())

    def test_cross_family_rejected(self):
        for family in (ClientFamily.CLASSIC, ClientFamily.FOREVER):
            with self.subTest(family=family), self.assertRaises(ValueError):
                require_same_client(self.identity(), self.identity(family))

    def test_build_interface_and_variant_boundaries(self):
        for kwargs in ({"build": "124"}, {"interface": 120106}, {"variant": "era"}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                require_same_client(self.identity(), self.identity(**kwargs))

    def test_unknown_or_missing_identity_rejected(self):
        for kwargs in ({"family": "retail"}, {"build": " "}, {"variant": ""}, {"interface": True}, {"interface": 0}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.identity(**kwargs)


if __name__ == "__main__":
    unittest.main()
