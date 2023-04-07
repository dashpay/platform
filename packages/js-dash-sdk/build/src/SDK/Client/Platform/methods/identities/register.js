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
var broadcastStateTransition_1 = __importDefault(require("../../broadcastStateTransition"));
var wait_1 = require("../../../../../utils/wait");
/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=1000000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
function register(fundingAmount) {
    if (fundingAmount === void 0) { fundingAmount = 1000000; }
    return __awaiter(this, void 0, void 0, function () {
        var client, account, _a, assetLockTransaction, assetLockPrivateKey, assetLockOutputIndex, assetLockProof, _b, identity, identityCreateTransition, identityIndex, maxAttempts, attempt, registeredIdentity;
        return __generator(this, function (_c) {
            switch (_c.label) {
                case 0:
                    this.logger.debug("[Identity#register] Register identity with funding amount " + fundingAmount);
                    return [4 /*yield*/, this.initialize()];
                case 1:
                    _c.sent();
                    client = this.client;
                    return [4 /*yield*/, client.getWalletAccount()];
                case 2:
                    account = _c.sent();
                    return [4 /*yield*/, this.identities.utils.createAssetLockTransaction(fundingAmount)];
                case 3:
                    _a = _c.sent(), assetLockTransaction = _a.transaction, assetLockPrivateKey = _a.privateKey, assetLockOutputIndex = _a.outputIndex;
                    // Broadcast Asset Lock transaction
                    return [4 /*yield*/, account.broadcastTransaction(assetLockTransaction)];
                case 4:
                    // Broadcast Asset Lock transaction
                    _c.sent();
                    this.logger.silly("[Identity#register] Broadcasted asset lock transaction \"" + assetLockTransaction.hash + "\"");
                    return [4 /*yield*/, this.identities.utils
                            .createAssetLockProof(assetLockTransaction, assetLockOutputIndex)];
                case 5:
                    assetLockProof = _c.sent();
                    this.logger.silly("[Identity#register] Created asset lock proof with tx \"" + assetLockTransaction.hash + "\"");
                    return [4 /*yield*/, this.identities.utils
                            .createIdentityCreateTransition(assetLockProof, assetLockPrivateKey)];
                case 6:
                    _b = _c.sent(), identity = _b.identity, identityCreateTransition = _b.identityCreateTransition, identityIndex = _b.identityIndex;
                    this.logger.silly("[Identity#register] Created IdentityCreateTransition with asset lock tx \"" + assetLockTransaction.hash + "\"");
                    // TODO: add skipValidation flag?
                    //  Basic validation already happening in createIdentityCreateTransition
                    return [4 /*yield*/, broadcastStateTransition_1.default(this, identityCreateTransition)];
                case 7:
                    // TODO: add skipValidation flag?
                    //  Basic validation already happening in createIdentityCreateTransition
                    _c.sent();
                    this.logger.silly('[Identity#register] Broadcasted IdentityCreateTransition');
                    // If state transition was broadcast without any errors, import identity to the account
                    account.storage
                        .getWalletStore(account.walletId)
                        .insertIdentityIdAtIndex(identity.getId().toString(), identityIndex);
                    maxAttempts = 20;
                    attempt = 0;
                    registeredIdentity = null;
                    _c.label = 8;
                case 8:
                    if (!(registeredIdentity === null && attempt < maxAttempts)) return [3 /*break*/, 11];
                    return [4 /*yield*/, wait_1.wait(100)];
                case 9:
                    _c.sent();
                    return [4 /*yield*/, this.identities.get(identity.getId())];
                case 10:
                    registeredIdentity = _c.sent();
                    attempt += 1;
                    return [3 /*break*/, 8];
                case 11:
                    if (registeredIdentity === null) {
                        throw new Error("Can't fetch created identity with id " + identity.getId());
                    }
                    // We cannot just return registeredIdentity as we want to
                    // keep additional information (assetLockProof and transaction) instance
                    identity.setMetadata(registeredIdentity.getMetadata());
                    identity.setBalance(registeredIdentity.getBalance());
                    identity.setPublicKeys(registeredIdentity.getPublicKeys());
                    this.logger.debug("[Identity#register] Registered identity \"" + identity.getId() + "\"");
                    return [2 /*return*/, identity];
            }
        });
    });
}
exports.default = register;
//# sourceMappingURL=register.js.map