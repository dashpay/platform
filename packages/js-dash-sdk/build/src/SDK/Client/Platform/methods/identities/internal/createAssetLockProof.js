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
Object.defineProperty(exports, "__esModule", { value: true });
exports.createAssetLockProof = void 0;
var _a = require('@dashevo/wallet-lib/src/errors'), InstantLockTimeoutError = _a.InstantLockTimeoutError, TxMetadataTimeoutError = _a.TxMetadataTimeoutError;
/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {Transaction} assetLockTransaction
 * @param {number} outputIndex - index of the funding output in the asset lock transaction
 * @return {AssetLockProof} - asset lock proof to be used in the state transition
 * that can be used to sign registration/top-up state transition
 */
function createAssetLockProof(assetLockTransaction, outputIndex) {
    return __awaiter(this, void 0, void 0, function () {
        var platform, account, wasmDpp, _a, instantLockPromise, cancelInstantLock, _b, txMetadataPromise, cancelTxMetadata, cancelObtainCoreChainLockedHeight, rejectTimer, rejectionTimeout;
        return __generator(this, function (_c) {
            switch (_c.label) {
                case 0:
                    platform = this;
                    return [4 /*yield*/, platform.initialize()];
                case 1:
                    _c.sent();
                    return [4 /*yield*/, platform.client.getWalletAccount()];
                case 2:
                    account = _c.sent();
                    wasmDpp = platform.wasmDpp;
                    _a = account.waitForInstantLock(assetLockTransaction.hash), instantLockPromise = _a.promise, cancelInstantLock = _a.cancel;
                    _b = account.waitForTxMetadata(assetLockTransaction.hash), txMetadataPromise = _b.promise, cancelTxMetadata = _b.cancel;
                    rejectionTimeout = account.waitForTxMetadataTimeout > account.waitForInstantLockTimeout
                        // wait for platform to sync core chain locked height
                        // @ts-ignore
                        ? account.waitForTxMetadataTimeout + 360000
                        // @ts-ignore
                        : account.waitForInstantLockTimeout;
                    return [2 /*return*/, Promise.race([
                            // Wait for Instant Lock
                            instantLockPromise
                                .then(function (instantLock) {
                                clearTimeout(rejectTimer);
                                cancelTxMetadata();
                                if (cancelObtainCoreChainLockedHeight) {
                                    cancelObtainCoreChainLockedHeight();
                                }
                                // @ts-ignore
                                return wasmDpp.identity.createInstantAssetLockProof(instantLock.toBuffer(), assetLockTransaction.toBuffer(), outputIndex);
                            })
                                .catch(function (error) {
                                if (error instanceof InstantLockTimeoutError) {
                                    // Instant Lock is timed out.
                                    // Allow chain proof to win the race
                                    return new Promise(function () { });
                                }
                                return Promise.reject(error);
                            }),
                            // Wait for transaction is mined and platform chain synced core height to the transaction height
                            txMetadataPromise
                                .then(function (assetLockMetadata) { return platform.identities.utils
                                // @ts-ignore
                                .waitForCoreChainLockedHeight(assetLockMetadata.height)
                                .then(function (_a) {
                                var promise = _a.promise, cancel = _a.cancel;
                                cancelObtainCoreChainLockedHeight = cancel;
                                return promise;
                            })
                                .then(function () {
                                clearTimeout(rejectTimer);
                                cancelInstantLock();
                                // Change endianness of raw txId bytes in outPoint to match expectations of dashcore-rust
                                var outPointBuffer = assetLockTransaction.getOutPointBuffer(outputIndex);
                                var txIdBuffer = outPointBuffer.slice(0, 32);
                                var outputIndexBuffer = outPointBuffer.slice(32);
                                txIdBuffer.reverse();
                                outPointBuffer = Buffer.concat([txIdBuffer, outputIndexBuffer]);
                                // @ts-ignore
                                return wasmDpp.identity.createChainAssetLockProof(
                                // @ts-ignore
                                assetLockMetadata.height, outPointBuffer);
                            }); })
                                .catch(function (error) {
                                if (error instanceof TxMetadataTimeoutError) {
                                    // Instant Lock is timed out.
                                    // Allow instant proof to win the race
                                    return new Promise(function () { });
                                }
                                return Promise.reject(error);
                            }),
                            // Common timeout for getting proofs
                            new Promise(function (_, reject) {
                                rejectTimer = setTimeout(function () {
                                    cancelTxMetadata();
                                    if (cancelObtainCoreChainLockedHeight) {
                                        cancelObtainCoreChainLockedHeight();
                                    }
                                    cancelInstantLock();
                                    reject(new Error('Asset Lock Proof creation timeout'));
                                }, rejectionTimeout);
                            }),
                        ])];
            }
        });
    });
}
exports.createAssetLockProof = createAssetLockProof;
exports.default = createAssetLockProof;
//# sourceMappingURL=createAssetLockProof.js.map