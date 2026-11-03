import unittest
from deploy.check_gateway_auth import validate


class GatewayAuthTests(unittest.TestCase):
    def test_exact_token_required(self):
        token = "a" * 64
        validate("MCP_GATEWAY_AUTH_TOKEN=" + token + "\n", token)
        for contents, environment in [
            ("", token),
            ("MCP_GATEWAY_AUTH_TOKEN=" + token + "\n", "b" * 64),
            ("MCP_GATEWAY_AUTH_TOKEN=" + token + "\nEXTRA=x\n", token),
            ("MCP_GATEWAY_AUTH_TOKEN=short\n", "short"),
        ]:
            with self.assertRaises(ValueError):
                validate(contents, environment)
