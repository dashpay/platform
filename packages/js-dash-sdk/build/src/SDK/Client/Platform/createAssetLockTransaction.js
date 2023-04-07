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
exports.createAssetLockTransaction = void 0;
var dashcore_lib_1 = require("@dashevo/dashcore-lib");
var wallet_lib_1 = require("@dashevo/wallet-lib");
// We're creating a new transaction every time and the index is always 0
var ASSET_LOCK_OUTPUT_INDEX = 0;
/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {number} fundingAmount - amount of dash to fund the identity's credits
 * @return {Promise<{transaction: Transaction, privateKey: PrivateKey}>}
 *  - transaction and one time private key
 * that can be used to sign registration/top-up state transition
 */
function createAssetLockTransaction(fundingAmount) {
    return __awaiter(this, void 0, void 0, function () {
        var platform, account, dppModule, assetLockOneTimePrivateKey, assetLockOneTimePublicKey, identityAddress, changeAddress, lockTransaction, output, utxos, balance, selection, utxoAddresses, utxoHDPrivateKey, signingKeys, transaction;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    platform = this;
                    return [4 /*yield*/, platform.client.getWalletAccount()];
                case 1:
                    account = _a.sent();
                    dppModule = platform.dppModule;
                    assetLockOneTimePrivateKey = new dashcore_lib_1.PrivateKey(dppModule.generateTemporaryEcdsaPrivateKey());
                    assetLockOneTimePublicKey = assetLockOneTimePrivateKey.toPublicKey();
                    identityAddress = assetLockOneTimePublicKey.toAddress(platform.client.network).toString();
                    changeAddress = account.getUnusedAddress('internal').address;
                    lockTransaction = new dashcore_lib_1.Transaction(undefined);
                    output = {
                        satoshis: fundingAmount,
                        address: identityAddress,
                    };
                    utxos = account.getUTXOS();
                    balance = account.getTotalBalance();
                    if (balance < output.satoshis) {
                        throw new Error("Not enough balance (" + balance + ") to cover burn amount of " + fundingAmount);
                    }
                    selection = wallet_lib_1.utils.coinSelection(utxos, [output]);
                    lockTransaction
                        .from(selection.utxos)
                        // @ts-ignore
                        // eslint-disable-next-line
                        .addBurnOutput(output.satoshis, assetLockOneTimePublicKey._getID())
                        .change(changeAddress);
                    utxoAddresses = selection.utxos.map(function (utxo) { return utxo.address.toString(); });
                    utxoHDPrivateKey = account.getPrivateKeys(utxoAddresses);
                    signingKeys = utxoHDPrivateKey.map(function (hdprivateKey) { return hdprivateKey.privateKey; });
                    transaction = lockTransaction.sign(signingKeys);
                    return [2 /*return*/, {
                            transaction: transaction,
                            privateKey: assetLockOneTimePrivateKey,
                            outputIndex: ASSET_LOCK_OUTPUT_INDEX,
                        }];
            }
        });
    });
}
exports.createAssetLockTransaction = createAssetLockTransaction;
exports.default = createAssetLockTransaction;
//# sourceMappingURL=createAssetLockTransaction.js.map