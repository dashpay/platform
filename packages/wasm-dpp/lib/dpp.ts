// import * as dpp_module from '../wasm/wasm_dpp';
import {IdentityPublicKey as UnprocessedIdentityPublicKey, KeyType, KeySecurityLevel, KeyPurpose} from "../wasm/wasm_dpp";

export class IdentityPublicKey extends UnprocessedIdentityPublicKey {
    static TYPES = KeyType;
    static PURPOSES = KeyPurpose;
    static SECURITY_LEVELS = KeySecurityLevel;
}

export * from '../wasm/wasm_dpp';