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
var chai_1 = require("chai");
var dashcore_lib_1 = require("@dashevo/dashcore-lib");
var stateTransitionTypes_1 = __importDefault(require("@dashevo/dpp/lib/stateTransition/stateTransitionTypes"));
var wasm_dpp_1 = __importDefault(require("@dashevo/wasm-dpp"));
var getResponseMetadataFixture_1 = __importDefault(require("../../test/fixtures/getResponseMetadataFixture"));
var index_1 = require("./index");
require("mocha");
var createFakeIntantLock_1 = require("../../utils/createFakeIntantLock");
var StateTransitionBroadcastError_1 = require("../../errors/StateTransitionBroadcastError");
var createIdentityFixtureInAccount_1 = require("../../test/fixtures/createIdentityFixtureInAccount");
var createAndAttachTransportMocksToClient_1 = require("../../test/mocks/createAndAttachTransportMocksToClient");
var createTransactionFixtureInAccount_1 = require("../../test/fixtures/createTransactionFixtureInAccount");
// @ts-ignore
var getDocumentsFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDocumentsFixture');
// @ts-ignore
var getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');
var GetDataContractResponse = require('@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse');
var blockHeaderFixture = '00000020e2bddfb998d7be4cc4c6b126f04d6e4bd201687523ded527987431707e0200005520320b4e263bec33e08944656f7ce17efbc2c60caab7c8ed8a73d413d02d3a169d555ecdd6021e56d000000203000500010000000000000000000000000000000000000000000000000000000000000000ffffffff050219250102ffffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac0000000046020019250000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f8000000000000000000000000000000000000000000000000000000000000000003000600000000000000fd4901010019250000010001d02e9ee1b14c022ad6895450f3375a8e9a87f214912d4332fa997996d2000000320000000000000032000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000';
var privateKeyFixture = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';
var IdentityPublicKey;
var IdentityPublicKeyWithWitness;
describe('Dash - Client', function suite() {
    var _this = this;
    this.timeout(30000);
    var testMnemonic;
    var transportMock;
    var testHDKey;
    var client;
    var account;
    var dapiClientMock;
    var identityFixture;
    var documentsFixture;
    var dataContractFixture;
    before(function () { return __awaiter(_this, void 0, void 0, function () {
        var _a;
        return __generator(this, function (_b) {
            switch (_b.label) {
                case 0: return [4 /*yield*/, wasm_dpp_1.default()];
                case 1:
                    // TODO(wasm): expose primitives by dedicated module?
                    (_a = _b.sent(), IdentityPublicKey = _a.IdentityPublicKey, IdentityPublicKeyWithWitness = _a.IdentityPublicKeyWithWitness);
                    return [2 /*return*/];
            }
        });
    }); });
    beforeEach(function beforeEach() {
        return __awaiter(this, void 0, void 0, function () {
            var _a;
            return __generator(this, function (_b) {
                switch (_b.label) {
                    case 0:
                        testMnemonic = 'agree country attract master mimic ball load beauty join gentle turtle hover';
                        testHDKey = 'tprv8ZgxMBicQKsPeGi4CikhacVPz6UmErenu1PoD3S4XcEDSPP8auRaS8hG3DQtsQ2i9HACgohHwF5sgMVJNksoKqYoZbis8o75Pp1koCme2Yo';
                        client = new index_1.Client({
                            wallet: {
                                HDPrivateKey: testHDKey,
                            },
                        });
                        return [4 /*yield*/, createAndAttachTransportMocksToClient_1.createAndAttachTransportMocksToClient(client, this.sinon)];
                    case 1:
                        (_a = _b.sent(), transportMock = _a.transportMock, dapiClientMock = _a.dapiClientMock);
                        return [4 /*yield*/, client.getWalletAccount()];
                    case 2:
                        account = _b.sent();
                        // add fake tx to the wallet so it will be able to create transactions
                        return [4 /*yield*/, createTransactionFixtureInAccount_1.createTransactionInAccount(account)];
                    case 3:
                        // add fake tx to the wallet so it will be able to create transactions
                        _b.sent();
                        return [4 /*yield*/, createIdentityFixtureInAccount_1.createIdentityFixtureInAccount(account)];
                    case 4:
                        // create an identity in the account so we can sign state transitions
                        identityFixture = _b.sent();
                        return [4 /*yield*/, getDataContractFixture()];
                    case 5:
                        dataContractFixture = _b.sent();
                        return [4 /*yield*/, getDocumentsFixture(dataContractFixture)];
                    case 6:
                        documentsFixture = _b.sent();
                        transportMock.getTransaction.resolves({
                            transaction: new dashcore_lib_1.Transaction('03000000019ecd68f367aba679209b9c912ff1d2ef9147f90eba2a47b5fb0158e27fb15476000000006b483045022100af2ca966eaeef8f5493fd8bcf2248d60b3f6b8236c137e2d099c8ba35878bf9402204f653232768eb8b06969b13f0aa3579d653163f757009e0c261c9ffd32332ffb0121034244016aa525c632408bc627923590cf136b47035cd57aa6f1fa8b696d717304ffffffff021027000000000000166a140f177a991f37fe6cbb08fb3f21b9629fa47330e3a85b0100000000001976a914535c005bfef672162aa2c53f0f6630a57ade344588ac00000000'),
                            blockHash: Buffer.from('0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', 'hex'),
                            height: 42,
                            confirmations: 10,
                            isInstantLocked: true,
                            isChainLocked: false,
                        });
                        transportMock.getBlockHeaderByHash
                            .returns(dashcore_lib_1.BlockHeader.fromString(blockHeaderFixture));
                        dapiClientMock.platform.getDataContract
                            .resolves(new GetDataContractResponse(dataContractFixture.toBuffer(), getResponseMetadataFixture_1.default()));
                        return [2 /*return*/];
                }
            });
        });
    });
    it('should provide expected class', function () {
        chai_1.expect(index_1.Client.name).to.be.equal('Client');
        chai_1.expect(index_1.Client.constructor.name).to.be.equal('Function');
    });
    it('should be instantiable', function () {
        client = new index_1.Client();
        chai_1.expect(client).to.exist;
        chai_1.expect(client.network).to.be.equal('testnet');
        chai_1.expect(client.getDAPIClient().constructor.name).to.be.equal('DAPIClient');
    });
    it('should not initiate wallet lib without mnemonic', function () {
        client = new index_1.Client();
        chai_1.expect(client.wallet).to.be.equal(undefined);
    });
    it('should initiate wallet-lib with a mnemonic', function () { return __awaiter(_this, void 0, void 0, function () {
        var _a, _b;
        return __generator(this, function (_c) {
            switch (_c.label) {
                case 0:
                    client = new index_1.Client({
                        wallet: {
                            mnemonic: testMnemonic,
                            offlineMode: true,
                        },
                    });
                    chai_1.expect(client.wallet).to.exist;
                    chai_1.expect(client.wallet.offlineMode).to.be.equal(true);
                    return [4 /*yield*/, ((_a = client.wallet) === null || _a === void 0 ? void 0 : _a.storage.stopWorker())];
                case 1:
                    _c.sent();
                    return [4 /*yield*/, ((_b = client.wallet) === null || _b === void 0 ? void 0 : _b.disconnect())];
                case 2:
                    _c.sent();
                    return [4 /*yield*/, client.getWalletAccount()];
                case 3:
                    account = _c.sent();
                    return [4 /*yield*/, account.disconnect()];
                case 4:
                    _c.sent();
                    return [2 /*return*/];
            }
        });
    }); });
    it('should throw an error if client and wallet have different networks', function () { return __awaiter(_this, void 0, void 0, function () {
        return __generator(this, function (_a) {
            try {
                // eslint-disable-next-line
                new index_1.Client({
                    network: 'testnet',
                    wallet: {
                        mnemonic: testMnemonic,
                        offlineMode: true,
                        network: 'evonet',
                    },
                });
                chai_1.expect.fail('should throw an error');
            }
            catch (e) {
                chai_1.expect(e.message).to.equal('Wallet and Client networks are different');
            }
            return [2 /*return*/];
        });
    }); });
    describe('#platform.identities.register ', function () { return __awaiter(_this, void 0, void 0, function () {
        var _this = this;
        return __generator(this, function (_a) {
            it('should register an identity', function () { return __awaiter(_this, void 0, void 0, function () {
                var accountIdentitiesCountBeforeTest, identity, serializedSt, interceptedIdentityStateTransition, interceptedAssetLockProof, transaction, isLock, importedIdentityIds;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0:
                            accountIdentitiesCountBeforeTest = account.identities.getIdentityIds().length;
                            return [4 /*yield*/, client.platform.identities.register()];
                        case 1:
                            identity = _a.sent();
                            chai_1.expect(identity).to.be.not.null;
                            serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(0).args[0];
                            return [4 /*yield*/, client
                                    .platform.wasmDpp.stateTransition.createFromBuffer(serializedSt)];
                        case 2:
                            interceptedIdentityStateTransition = _a.sent();
                            interceptedAssetLockProof = interceptedIdentityStateTransition.getAssetLockProof();
                            transaction = new dashcore_lib_1.Transaction(transportMock.sendTransaction.getCall(0).args[0]);
                            isLock = createFakeIntantLock_1.createFakeInstantLock(transaction.hash);
                            // Check intercepted st
                            chai_1.expect(interceptedAssetLockProof.getInstantLock()).to.be.deep.equal(isLock.toBuffer());
                            chai_1.expect(interceptedAssetLockProof.getTransaction()).to.be.deep.equal(transaction.toBuffer());
                            importedIdentityIds = account.identities.getIdentityIds();
                            // Check that we've imported identities properly
                            chai_1.expect(importedIdentityIds.length).to.be.equal(accountIdentitiesCountBeforeTest + 1);
                            chai_1.expect(importedIdentityIds[1]).to.be
                                .equal(interceptedIdentityStateTransition.getIdentityId().toString());
                            return [2 /*return*/];
                    }
                });
            }); });
            it('should throw TransitionBroadcastError when transport resolves error', function () { return __awaiter(_this, void 0, void 0, function () {
                var accountIdentitiesCountBeforeTest, errorResponse, error, e_1, importedIdentityIds;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0:
                            accountIdentitiesCountBeforeTest = account.identities.getIdentityIds().length;
                            errorResponse = {
                                error: {
                                    code: 2,
                                    message: 'Error happened',
                                    data: {},
                                },
                            };
                            dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);
                            _a.label = 1;
                        case 1:
                            _a.trys.push([1, 3, , 4]);
                            return [4 /*yield*/, client.platform.identities.register()];
                        case 2:
                            _a.sent();
                            return [3 /*break*/, 4];
                        case 3:
                            e_1 = _a.sent();
                            error = e_1;
                            return [3 /*break*/, 4];
                        case 4:
                            chai_1.expect(error).to.be.an.instanceOf(StateTransitionBroadcastError_1.StateTransitionBroadcastError);
                            chai_1.expect(error.getCode()).to.be.equal(errorResponse.error.code);
                            chai_1.expect(error.getMessage()).to.be.equal(errorResponse.error.message);
                            importedIdentityIds = account.identities.getIdentityIds();
                            // Check that no identities were imported
                            chai_1.expect(importedIdentityIds.length).to.be.equal(accountIdentitiesCountBeforeTest);
                            return [2 /*return*/];
                    }
                });
            }); });
            return [2 /*return*/];
        });
    }); });
    describe('#platform.identities.topUp', function () { return __awaiter(_this, void 0, void 0, function () {
        var _this = this;
        return __generator(this, function (_a) {
            it('should top up an identity', function () { return __awaiter(_this, void 0, void 0, function () {
                var identity, serializedSt, interceptedIdentityStateTransition, interceptedAssetLockProof, transaction, isLock;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0: return [4 /*yield*/, client.platform.identities.register(1000)];
                        case 1:
                            identity = _a.sent();
                            // Topping up the identity
                            return [4 /*yield*/, client.platform.identities.topUp(identity.getId(), 1000)];
                        case 2:
                            // Topping up the identity
                            _a.sent();
                            chai_1.expect(identity).to.be.not.null;
                            serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(1).args[0];
                            return [4 /*yield*/, client
                                    .platform.wasmDpp.stateTransition.createFromBuffer(serializedSt)];
                        case 3:
                            interceptedIdentityStateTransition = _a.sent();
                            interceptedAssetLockProof = interceptedIdentityStateTransition.getAssetLockProof();
                            chai_1.expect(interceptedIdentityStateTransition.getType())
                                .to.be.equal(stateTransitionTypes_1.default.IDENTITY_TOP_UP);
                            transaction = new dashcore_lib_1.Transaction(transportMock.sendTransaction.getCall(1).args[0]);
                            isLock = createFakeIntantLock_1.createFakeInstantLock(transaction.hash);
                            // Check intercepted st
                            chai_1.expect(interceptedAssetLockProof.getInstantLock()).to.be
                                .deep.equal(isLock.toBuffer());
                            chai_1.expect(interceptedAssetLockProof.getTransaction()).to.be
                                .deep.equal(transaction.toBuffer());
                            return [2 /*return*/];
                    }
                });
            }); });
            it('should throw TransitionBroadcastError when transport resolves error', function () { return __awaiter(_this, void 0, void 0, function () {
                var identity, errorResponse, error, e_2;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0: return [4 /*yield*/, client.platform.identities.register(10000)];
                        case 1:
                            identity = _a.sent();
                            errorResponse = {
                                error: {
                                    code: 2,
                                    message: 'Error happened',
                                    data: {},
                                },
                            };
                            dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);
                            _a.label = 2;
                        case 2:
                            _a.trys.push([2, 4, , 5]);
                            // Topping up the identity
                            return [4 /*yield*/, client.platform.identities.topUp(identity.getId(), 10000)];
                        case 3:
                            // Topping up the identity
                            _a.sent();
                            return [3 /*break*/, 5];
                        case 4:
                            e_2 = _a.sent();
                            error = e_2;
                            return [3 /*break*/, 5];
                        case 5:
                            chai_1.expect(error).to.be.an.instanceOf(StateTransitionBroadcastError_1.StateTransitionBroadcastError);
                            chai_1.expect(error.getCode()).to.be.equal(errorResponse.error.code);
                            chai_1.expect(error.getMessage()).to.be.equal(errorResponse.error.message);
                            return [2 /*return*/];
                    }
                });
            }); });
            return [2 /*return*/];
        });
    }); });
    describe('#platform.identities.update', function () { return __awaiter(_this, void 0, void 0, function () {
        var _this = this;
        return __generator(this, function (_a) {
            it('should update an identity', function () { return __awaiter(_this, void 0, void 0, function () {
                var identity, privateKey, publicKeysToAdd, publicKeysToDisable, serializedSt, interceptedIdentityStateTransition, publicKeysAdded, publicKeysDisabled;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0: return [4 /*yield*/, client.platform.identities.register(1000)];
                        case 1:
                            identity = _a.sent();
                            privateKey = new dashcore_lib_1.PrivateKey(privateKeyFixture);
                            publicKeysToAdd = [
                                new IdentityPublicKeyWithWitness({
                                    id: 3,
                                    type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
                                    data: privateKey.toPublicKey().toBuffer(),
                                    purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
                                    securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
                                    readOnly: false,
                                    signature: Buffer.alloc(0),
                                }),
                            ];
                            publicKeysToDisable = [identity.getPublicKeys()[0]];
                            // Updating the identity
                            return [4 /*yield*/, client.platform.identities.update(identity, {
                                    add: publicKeysToAdd,
                                    disable: publicKeysToDisable,
                                }, {
                                    3: privateKey,
                                })];
                        case 2:
                            // Updating the identity
                            _a.sent();
                            chai_1.expect(identity).to.be.not.null;
                            serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(1).args[0];
                            return [4 /*yield*/, client
                                    .platform.wasmDpp.stateTransition.createFromBuffer(serializedSt)];
                        case 3:
                            interceptedIdentityStateTransition = _a.sent();
                            chai_1.expect(interceptedIdentityStateTransition.getType())
                                .to.be.equal(stateTransitionTypes_1.default.IDENTITY_UPDATE);
                            publicKeysAdded = interceptedIdentityStateTransition.getPublicKeysToAdd();
                            chai_1.expect(publicKeysAdded.map(function (key) { return key.toObject({ skipSignature: true }); }))
                                .to.deep.equal(publicKeysToAdd.map(function (key) { return key.toObject({ skipSignature: true }); }));
                            publicKeysDisabled = interceptedIdentityStateTransition.getPublicKeyIdsToDisable();
                            chai_1.expect(publicKeysDisabled).to.deep.equal(publicKeysToDisable.map(function (key) { return key.getId(); }));
                            return [2 /*return*/];
                    }
                });
            }); });
            it('should throw TransitionBroadcastError when transport resolves error', function () { return __awaiter(_this, void 0, void 0, function () {
                var identity, errorResponse, error, e_3;
                return __generator(this, function (_a) {
                    switch (_a.label) {
                        case 0: return [4 /*yield*/, client.platform.identities.register(10000)];
                        case 1:
                            identity = _a.sent();
                            errorResponse = {
                                error: {
                                    code: 2,
                                    message: 'Error happened',
                                    data: {},
                                },
                            };
                            dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);
                            _a.label = 2;
                        case 2:
                            _a.trys.push([2, 4, , 5]);
                            // Topping up the identity
                            return [4 /*yield*/, client.platform.identities.topUp(identity.getId(), 10000)];
                        case 3:
                            // Topping up the identity
                            _a.sent();
                            return [3 /*break*/, 5];
                        case 4:
                            e_3 = _a.sent();
                            error = e_3;
                            return [3 /*break*/, 5];
                        case 5:
                            chai_1.expect(error).to.be.an.instanceOf(StateTransitionBroadcastError_1.StateTransitionBroadcastError);
                            chai_1.expect(error.getCode()).to.be.equal(errorResponse.error.code);
                            chai_1.expect(error.getMessage()).to.be.equal(errorResponse.error.message);
                            return [2 /*return*/];
                    }
                });
            }); });
            return [2 /*return*/];
        });
    }); });
    describe('#platform.documents.broadcast', function () {
        it('should throw TransitionBroadcastError when transport resolves error', function () { return __awaiter(_this, void 0, void 0, function () {
            var errorResponse, error, e_4;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0:
                        errorResponse = {
                            error: {
                                code: 2,
                                message: 'Error happened',
                                data: {},
                            },
                        };
                        dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);
                        _a.label = 1;
                    case 1:
                        _a.trys.push([1, 3, , 4]);
                        return [4 /*yield*/, client.platform.documents.broadcast({
                                create: documentsFixture,
                            }, identityFixture)];
                    case 2:
                        _a.sent();
                        return [3 /*break*/, 4];
                    case 3:
                        e_4 = _a.sent();
                        error = e_4;
                        return [3 /*break*/, 4];
                    case 4:
                        chai_1.expect(error).to.be.an.instanceOf(StateTransitionBroadcastError_1.StateTransitionBroadcastError);
                        chai_1.expect(error.getCode()).to.be.equal(errorResponse.error.code);
                        chai_1.expect(error.getMessage()).to.be.equal(errorResponse.error.message);
                        return [2 /*return*/];
                }
            });
        }); });
        it('should broadcast documents', function () { return __awaiter(_this, void 0, void 0, function () {
            var proofResponse, serializedSt, interceptedSt, _a, documentTransitions;
            return __generator(this, function (_b) {
                switch (_b.label) {
                    case 0:
                        proofResponse = {
                            proof: {},
                        };
                        dapiClientMock.platform.waitForStateTransitionResult.resolves(proofResponse);
                        return [4 /*yield*/, client.platform.documents.broadcast({
                                create: documentsFixture,
                            }, identityFixture)];
                    case 1:
                        _b.sent();
                        serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(0).args[0];
                        return [4 /*yield*/, client
                                .platform.wasmDpp.stateTransition.createFromBuffer(serializedSt)];
                    case 2:
                        interceptedSt = _b.sent();
                        // .to.be.true() doesn't work after TS compilation in Chrome
                        _a = chai_1.expect;
                        return [4 /*yield*/, interceptedSt.verifySignature(identityFixture.getPublicKeyById(1))];
                    case 3:
                        // .to.be.true() doesn't work after TS compilation in Chrome
                        _a.apply(void 0, [_b.sent()]).to.be.equal(true);
                        documentTransitions = interceptedSt.getTransitions();
                        chai_1.expect(documentTransitions.length).to.be.greaterThan(0);
                        chai_1.expect(documentTransitions.length).to.be.equal(documentsFixture.length);
                        return [2 /*return*/];
                }
            });
        }); });
    });
    describe('#platform.contracts.publish', function () {
        it('should throw TransitionBroadcastError when transport resolves error', function () { return __awaiter(_this, void 0, void 0, function () {
            var errorResponse, error, e_5;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0:
                        errorResponse = {
                            error: {
                                code: 2,
                                message: 'Error happened',
                                data: {},
                            },
                        };
                        dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);
                        _a.label = 1;
                    case 1:
                        _a.trys.push([1, 3, , 4]);
                        return [4 /*yield*/, client.platform.contracts.publish(dataContractFixture, identityFixture)];
                    case 2:
                        _a.sent();
                        return [3 /*break*/, 4];
                    case 3:
                        e_5 = _a.sent();
                        error = e_5;
                        return [3 /*break*/, 4];
                    case 4:
                        chai_1.expect(error).to.be.an.instanceOf(StateTransitionBroadcastError_1.StateTransitionBroadcastError);
                        chai_1.expect(error.getCode()).to.be.equal(errorResponse.error.code);
                        chai_1.expect(error.getMessage()).to.be.equal(errorResponse.error.message);
                        return [2 /*return*/];
                }
            });
        }); });
        it('should broadcast data contract', function () { return __awaiter(_this, void 0, void 0, function () {
            var serializedSt, interceptedSt, _a;
            return __generator(this, function (_b) {
                switch (_b.label) {
                    case 0:
                        dapiClientMock.platform.waitForStateTransitionResult.resolves({
                            proof: {},
                        });
                        return [4 /*yield*/, client.platform.contracts.publish(dataContractFixture, identityFixture)];
                    case 1:
                        _b.sent();
                        serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(0).args[0];
                        return [4 /*yield*/, client
                                .platform.wasmDpp.stateTransition.createFromBuffer(serializedSt)];
                    case 2:
                        interceptedSt = _b.sent();
                        // .to.be.true() doesn't work after TS compilation in Chrome
                        _a = chai_1.expect;
                        return [4 /*yield*/, interceptedSt.verifySignature(identityFixture.getPublicKeyById(1))];
                    case 3:
                        // .to.be.true() doesn't work after TS compilation in Chrome
                        _a.apply(void 0, [_b.sent()]).to.be.equal(true);
                        chai_1.expect(interceptedSt.getEntropy()).to.be.deep.equal(dataContractFixture.getEntropy());
                        chai_1.expect(interceptedSt.getDataContract().toObject())
                            .to.be.deep.equal(dataContractFixture.toObject());
                        return [2 /*return*/];
                }
            });
        }); });
    });
});
//# sourceMappingURL=Client.spec.js.map