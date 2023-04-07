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
exports.createAndAttachTransportMocksToClient = void 0;
var dashcore_lib_1 = require("@dashevo/dashcore-lib");
var dapi_client_1 = __importDefault(require("@dashevo/dapi-client"));
var stateTransitionTypes_1 = __importDefault(require("@dashevo/dpp/lib/stateTransition/stateTransitionTypes"));
var wasm_dpp_1 = __importDefault(require("@dashevo/wasm-dpp"));
var createFakeIntantLock_1 = require("../../utils/createFakeIntantLock");
var getResponseMetadataFixture_1 = __importDefault(require("../fixtures/getResponseMetadataFixture"));
var createDapiClientMock_1 = require("./createDapiClientMock");
var wait_1 = require("../../utils/wait");
var GetIdentityResponse = require('@dashevo/dapi-client/lib/methods/platform/getIdentity/GetIdentityResponse');
// @ts-ignore
var TxStreamMock = require('@dashevo/wallet-lib/src/test/mocks/TxStreamMock');
// @ts-ignore
var TxStreamDataResponseMock = require('@dashevo/wallet-lib/src/test/mocks/TxStreamDataResponseMock');
// @ts-ignore
var TransportMock = require('@dashevo/wallet-lib/src/test/mocks/TransportMock');
function makeTxStreamEmitISLocksForTransactions(transportMock, txStreamMock) {
    transportMock.sendTransaction.callsFake(function (txString) {
        var transaction = new dashcore_lib_1.Transaction(txString);
        var isLock = createFakeIntantLock_1.createFakeInstantLock(transaction.hash);
        setImmediate(function () {
            // Emit IS lock for the transaction
            txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({ instantSendLockMessages: [isLock.toBuffer()] }));
        });
        // Emit the same transaction back to the client so it will know about the change transaction
        txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({ rawTransactions: [transaction.toBuffer()] }));
        return transaction.hash;
    });
}
/**
 * Makes stub remember the identity from the ST and respond with it
 * @param {Client} client
 * @param dapiClientMock
 */
function makeGetIdentityRespondWithIdentity(client, dapiClientMock, sinon) {
    return __awaiter(this, void 0, void 0, function () {
        var Identity;
        var _this = this;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0: return [4 /*yield*/, wasm_dpp_1.default()];
                case 1:
                    Identity = (_a.sent()).Identity;
                    dapiClientMock.platform.broadcastStateTransition.callsFake(function (stBuffer) { return __awaiter(_this, void 0, void 0, function () {
                        var interceptedIdentityStateTransition, identityToResolve_1;
                        return __generator(this, function (_a) {
                            switch (_a.label) {
                                case 0: return [4 /*yield*/, client
                                        .platform.wasmDpp.stateTransition.createFromBuffer(stBuffer)];
                                case 1:
                                    interceptedIdentityStateTransition = _a.sent();
                                    if (interceptedIdentityStateTransition.getType() === stateTransitionTypes_1.default.IDENTITY_CREATE) {
                                        identityToResolve_1 = new Identity({
                                            // TODO(wasm): get from platform.wasmDpp once we merge
                                            //  https://github.com/dashpay/platform/pull/841
                                            protocolVersion: 1,
                                            id: interceptedIdentityStateTransition.getIdentityId().toBuffer(),
                                            publicKeys: interceptedIdentityStateTransition
                                                .getPublicKeys().map(function (key) { return key.toObject({ skipSignature: true }); }),
                                            balance: interceptedIdentityStateTransition.getAssetLockProof().getOutput().satoshis,
                                            revision: 0,
                                        });
                                        dapiClientMock.platform.getIdentity.withArgs(sinon.match(function (id) { return id.equals(identityToResolve_1.getId().toBuffer()); }))
                                            .resolves(new GetIdentityResponse(identityToResolve_1.toBuffer(), getResponseMetadataFixture_1.default()));
                                    }
                                    return [2 /*return*/];
                            }
                        });
                    }); });
                    return [2 /*return*/];
            }
        });
    });
}
function createAndAttachTransportMocksToClient(client, sinon) {
    return __awaiter(this, void 0, void 0, function () {
        var txStreamMock, transportMock, dapiClientMock, accountPromise, blockHeadersProvider;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    txStreamMock = new TxStreamMock();
                    transportMock = new TransportMock(sinon, txStreamMock);
                    dapiClientMock = createDapiClientMock_1.createDapiClientMock(sinon);
                    // Mock wallet-lib transport to intercept transactions
                    client.wallet.transport = transportMock;
                    // Mock dapi client for platform endpoints
                    client.dapiClient = dapiClientMock;
                    accountPromise = client.wallet.getAccount();
                    // Breaking the event loop to emit an event
                    return [4 /*yield*/, wait_1.wait(0)];
                case 1:
                    // Breaking the event loop to emit an event
                    _a.sent();
                    blockHeadersProvider = client.wallet.transport.client.blockHeadersProvider;
                    blockHeadersProvider.emit(dapi_client_1.default.BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
                    return [4 /*yield*/, wait_1.wait(0)];
                case 2:
                    _a.sent();
                    // Emitting TX stream end event to mark finish of the tx sync
                    txStreamMock.emit(TxStreamMock.EVENTS.end);
                    // Wait for account to resolve
                    return [4 /*yield*/, accountPromise];
                case 3:
                    // Wait for account to resolve
                    _a.sent();
                    // Putting data in transport stubs
                    transportMock.getIdentitiesByPublicKeyHashes.resolves([]);
                    makeTxStreamEmitISLocksForTransactions(transportMock, txStreamMock);
                    return [4 /*yield*/, makeGetIdentityRespondWithIdentity(client, dapiClientMock, sinon)];
                case 4:
                    _a.sent();
                    return [2 /*return*/, { txStreamMock: txStreamMock, transportMock: transportMock, dapiClientMock: dapiClientMock }];
            }
        });
    });
}
exports.createAndAttachTransportMocksToClient = createAndAttachTransportMocksToClient;
//# sourceMappingURL=createAndAttachTransportMocksToClient.js.map