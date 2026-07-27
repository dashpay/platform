import { wasm } from '../../../dist/dpp.compressed.js';

/// Build a placeholder `SerializedOrchardAction` filled with deterministic
/// non-zero bytes. The cryptographic content is not verified at the wasm-dpp2
/// layer (that happens later in consensus validation), so wrappers accept any
/// well-shaped bytes for shape-level tests.
export function fakeOrchardAction(seed = 1): any {
  return new wasm.SerializedOrchardAction({
    nullifier: new Uint8Array(32).fill(seed),
    rk: new Uint8Array(32).fill(seed + 1),
    cmx: new Uint8Array(32).fill(seed + 2),
    encryptedNote: new Uint8Array(216).fill(seed + 3),
    cvNet: new Uint8Array(32).fill(seed + 4),
    spendAuthSig: new Uint8Array(64).fill(seed + 5),
  });
}

export const ZERO_ANCHOR = new Uint8Array(32);
export const ZERO_PROOF = new Uint8Array(256); // arbitrary placeholder size
export const ZERO_BINDING_SIG = new Uint8Array(64);
