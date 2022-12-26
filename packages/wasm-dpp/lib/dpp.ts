import * as dpp_module from '../wasm/wasm_dpp';
import { patchConsensusErrors } from './errors/patchConsensusErrors';

patchConsensusErrors();

// While we declared it above, those fields do not hold any values - let's assign them.
// We need to suppress the compiler here, as he won't be happy about those reassignments.
// @ts-ignore
dpp_module.IdentityPublicKey.TYPES = dpp_module.KeyType;
// @ts-ignore
dpp_module.IdentityPublicKey.PURPOSES = dpp_module.KeyPurpose;
// @ts-ignore
dpp_module.IdentityPublicKey.SECURITY_LEVELS = dpp_module.KeySecurityLevel;

export * from '../wasm/wasm_dpp';
export * from './errors/AbstractConsensusError';
export * from './errors/DPPError';

// Declarations written prior to "export *" will overwrite exports
export declare class IdentityPublicKey extends dpp_module.IdentityPublicKey {
    static TYPES: typeof dpp_module.KeyType;
    static PURPOSES: typeof dpp_module.KeyPurpose;
    static SECURITY_LEVELS: typeof dpp_module.KeySecurityLevel;
}
