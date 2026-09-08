"""``auth.hashing`` — argon2id password hashing (E1-S5, NFR-SEC-1).

Passwords are stored ONLY as an argon2id PHC string (embedded per-hash salt +
tuned params). Plaintext lives in a transient local ``str`` and is never logged
or persisted. Verification is constant-time (argon2-cffi). ``verify_dummy``
equalizes login timing when no account matches, to avoid user-enumeration.
"""

from __future__ import annotations

from argon2 import PasswordHasher as _Argon2Hasher
from argon2.exceptions import Argon2Error

PasswordHash = str


class PasswordHasher:
    """Thin wrapper over ``argon2.PasswordHasher`` (argon2id defaults)."""

    def __init__(self) -> None:
        self._ph = _Argon2Hasher()
        # A fixed hash used only to spend argon2 time on the account-miss path so
        # a bad email and a bad password take indistinguishable time.
        self._dummy = self._ph.hash("okc-web-dummy-password")

    def hash(self, plaintext: str) -> PasswordHash:
        return self._ph.hash(plaintext)

    def verify(self, plaintext: str, password_hash: PasswordHash) -> bool:
        """Constant-time verify. Returns ``False`` on any mismatch/invalid hash
        rather than raising, so callers branch on a bool."""
        try:
            return self._ph.verify(password_hash, plaintext)
        except Argon2Error:
            return False

    def verify_dummy(self, plaintext: str) -> None:
        """Spend argon2 time against a throwaway hash (account-miss timing parity)."""
        try:
            self._ph.verify(self._dummy, plaintext)
        except Argon2Error:
            pass
