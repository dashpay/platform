import * as dpp_module from '../wasm/wasm_dpp';
import _Identifier from "./identifier/Identifier";
import _IdentifierError from "./identifier/errors/IdentifierError";
export * from '../wasm/wasm_dpp';
export * from './errors/AbstractConsensusError';
export * from './errors/DPPError';
export declare class IdentityPublicKey extends dpp_module.IdentityPublicKey {
    static TYPES: typeof dpp_module.KeyType;
    static PURPOSES: typeof dpp_module.KeyPurpose;
    static SECURITY_LEVELS: typeof dpp_module.KeySecurityLevel;
}
export declare class Identifier extends _Identifier {
}
export declare class IdentifierError extends _IdentifierError {
}
