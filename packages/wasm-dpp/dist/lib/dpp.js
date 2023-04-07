"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    Object.defineProperty(o, k2, { enumerable: true, get: function() { return m[k]; } });
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !exports.hasOwnProperty(p)) __createBinding(exports, m, p);
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const dpp_module = __importStar(require("../wasm/wasm_dpp"));
const patchConsensusErrors_1 = require("./errors/patchConsensusErrors");
const Identifier_1 = __importDefault(require("./identifier/Identifier"));
const IdentifierError_1 = __importDefault(require("./identifier/errors/IdentifierError"));
patchConsensusErrors_1.patchConsensusErrors();
// While we declared it above, those fields do not hold any values - let's assign them.
// We need to suppress the compiler here, as he won't be happy about those reassignments.
// @ts-ignore
dpp_module.IdentityPublicKey.TYPES = dpp_module.KeyType;
// @ts-ignore
dpp_module.IdentityPublicKey.PURPOSES = dpp_module.KeyPurpose;
// @ts-ignore
dpp_module.IdentityPublicKey.SECURITY_LEVELS = dpp_module.KeySecurityLevel;
// @ts-ignore
dpp_module.Identifier = Identifier_1.default;
// @ts-ignore
dpp_module.IdentifierError = IdentifierError_1.default;
__exportStar(require("../wasm/wasm_dpp"), exports);
__exportStar(require("./errors/AbstractConsensusError"), exports);
__exportStar(require("./errors/DPPError"), exports);
