# Core quorum compatibility vectors

`core_quorums.json` freezes outputs from `dashpay/bls-signatures` at
`0842b17583888e8f46c252a4ee84cdfd58e0546f`, captured before its removal.
These synthetic keys are public test material.

Each case seeds rand 0.8.5 `StdRng` with 42, calls `PrivateKey::generate_dash_many`
for 2, 3 or 5 members, and calls `PrivateKey::threshold_recover` on those keys.
Member index `i` has a 32-byte ID with byte 0 equal to `i + 1`, byte 31 equal
to `i + 11`, and all other bytes zero. The asymmetric IDs pin Core's scalar
byte order. Signatures use the Basic scheme over `b"quorum compatibility"`.

The test checks generated member keys, the interpolated quorum key, and its
signature against these frozen values. It preserves an independent compatibility
oracle without linking or building the historical implementation.
