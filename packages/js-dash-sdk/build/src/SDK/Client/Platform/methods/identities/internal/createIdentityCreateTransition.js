"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (_) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createIdentityCreateTransition = void 0;
// TODO(wasm): replace with IdentityPublicKey from wasm-dpp
var IdentityPublicKey_1 = __importDefault(require("@dashevo/dpp/lib/identity/IdentityPublicKey"));
/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {AssetLockProof} assetLockProof - asset lock transaction proof
 *  for the identity create transition
 * @param {PrivateKey} assetLockPrivateKey - private key used in asset lock
 * @return {{identity: Identity, identityCreateTransition: IdentityCreateTransition}}
 *  - identity, state transition and index of the key used to create it
 * that can be used to sign registration/top-up state transition
 */
function createIdentityCreateTransition(assetLockProof, assetLockPrivateKey) {
    return __awaiter(this, void 0, void 0, function () {
        var platform, account, wasmDpp, identityIndex, identityMasterPrivateKey, identityMasterPublicKey, identitySecondPrivateKey, identitySecondPublicKey, identity, identityCreateTransition, _a, masterKey, secondKey, result;
        return __generator(this, function (_b) {
            switch (_b.label) {
                case 0:
                    platform = this;
                    return [4 /*yield*/, platform.initialize()];
                case 1:
                    _b.sent();
                    return [4 /*yield*/, platform.client.getWalletAccount()];
                case 2:
                    account = _b.sent();
                    wasmDpp = platform.wasmDpp;
                    return [4 /*yield*/, account.getUnusedIdentityIndex()];
                case 3:
                    identityIndex = _b.sent();
                    identityMasterPrivateKey = account.identities
                        .getIdentityHDKeyByIndex(identityIndex, 0).privateKey;
                    identityMasterPublicKey = identityMasterPrivateKey.toPublicKey();
                    identitySecondPrivateKey = account.identities
                        .getIdentityHDKeyByIndex(identityIndex, 1).privateKey;
                    identitySecondPublicKey = identitySecondPrivateKey.toPublicKey();
                    identity = wasmDpp.identity.create(assetLockProof, [{
                            id: 0,
                            data: identityMasterPublicKey.toBuffer(),
                            type: IdentityPublicKey_1.default.TYPES.ECDSA_SECP256K1,
                            purpose: IdentityPublicKey_1.default.PURPOSES.AUTHENTICATION,
                            securityLevel: IdentityPublicKey_1.default.SECURITY_LEVELS.MASTER,
                            readOnly: false,
                        },
                        {
                            id: 1,
                            data: identitySecondPublicKey.toBuffer(),
                            type: IdentityPublicKey_1.default.TYPES.ECDSA_SECP256K1,
                            purpose: IdentityPublicKey_1.default.PURPOSES.AUTHENTICATION,
                            securityLevel: IdentityPublicKey_1.default.SECURITY_LEVELS.HIGH,
                            readOnly: false,
                        },
                    ]);
                    identityCreateTransition = wasmDpp.identity.createIdentityCreateTransition(identity);
                    _a = identityCreateTransition.getPublicKeys(), masterKey = _a[0], secondKey = _a[1];
                    return [4 /*yield*/, identityCreateTransition
                            .signByPrivateKey(identityMasterPrivateKey.toBuffer(), IdentityPublicKey_1.default.TYPES.ECDSA_SECP256K1)];
                case 4:
                    _b.sent();
                    masterKey.setSignature(identityCreateTransition.getSignature());
                    identityCreateTransition.setSignature(undefined);
                    return [4 /*yield*/, identityCreateTransition
                            .signByPrivateKey(identitySecondPrivateKey.toBuffer(), IdentityPublicKey_1.default.TYPES.ECDSA_SECP256K1)];
                case 5:
                    _b.sent();
                    secondKey.setSignature(identityCreateTransition.getSignature());
                    identityCreateTransition.setSignature(undefined);
                    // Set public keys back after updating their signatures
                    identityCreateTransition.setPublicKeys([masterKey, secondKey]);
                    // Sign and validate state transition
                    return [4 /*yield*/, identityCreateTransition
                            .signByPrivateKey(assetLockPrivateKey.toBuffer(), IdentityPublicKey_1.default.TYPES.ECDSA_SECP256K1)];
                case 6:
                    // Sign and validate state transition
                    _b.sent();
                    return [4 /*yield*/, wasmDpp.stateTransition.validateBasic(identityCreateTransition)];
                case 7:
                    result = _b.sent();
                    if (!result.isValid()) {
                        // TODO(wasm): pretty print errors. JSON stringify is not handling wasm errors well
                        throw new Error("StateTransition is invalid - " + JSON.stringify(result.getErrors()));
                    }
                    return [2 /*return*/, { identity: identity, identityCreateTransition: identityCreateTransition, identityIndex: identityIndex }];
            }
        });
    });
}
exports.createIdentityCreateTransition = createIdentityCreateTransition;
exports.default = createIdentityCreateTransition;
//# sourceMappingURL=createIdentityCreateTransition.js.map