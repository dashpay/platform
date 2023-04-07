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
var getDataContractFixture_1 = __importDefault(require("@dashevo/dpp/lib/test/fixtures/getDataContractFixture"));
var generateRandomIdentifier_1 = __importDefault(require("@dashevo/dpp/lib/test/utils/generateRandomIdentifier"));
var createDPPMock_1 = __importDefault(require("@dashevo/dpp/lib/test/mocks/createDPPMock"));
var getDocumentsFixture_1 = __importDefault(require("@dashevo/dpp/lib/test/fixtures/getDocumentsFixture"));
var chai_1 = require("chai");
var getResponseMetadataFixture_1 = __importDefault(require("../../../../../test/fixtures/getResponseMetadataFixture"));
var get_1 = __importDefault(require("./get"));
var GetDocumentsResponse = require('@dashevo/dapi-client/lib/methods/platform/getDocuments/GetDocumentsResponse');
describe('Client - Platform - Documents - .get()', function () {
    var platform;
    var dataContract;
    var appDefinition;
    var getDocumentsMock;
    var appsGetMock;
    beforeEach(function beforeEach() {
        var _this = this;
        dataContract = getDataContractFixture_1.default();
        appDefinition = {
            contractId: dataContract.getId(),
            contract: dataContract,
        };
        getDocumentsMock = this.sinon.stub()
            .resolves(new GetDocumentsResponse([], getResponseMetadataFixture_1.default()));
        appsGetMock = this.sinon.stub().returns(appDefinition);
        platform = {
            dpp: createDPPMock_1.default(this.sinon),
            client: {
                getApps: function () { return ({
                    has: _this.sinon.stub().returns(true),
                    get: appsGetMock,
                }); },
                getDAPIClient: function () { return ({
                    platform: {
                        getDocuments: getDocumentsMock,
                    },
                }); },
            },
            initialize: this.sinon.stub(),
        };
    });
    it('should convert identifier properties inside where condition', function () { return __awaiter(void 0, void 0, void 0, function () {
        var id;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    id = generateRandomIdentifier_1.default();
                    return [4 /*yield*/, get_1.default.call(platform, 'app.withByteArrays', {
                            where: [
                                ['identifierField', '==', id.toString()],
                            ],
                        })];
                case 1:
                    _a.sent();
                    chai_1.expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
                        appDefinition.contractId,
                        'withByteArrays',
                        {
                            where: [
                                ['identifierField', '==', id],
                            ],
                        },
                    ]);
                    return [2 /*return*/];
            }
        });
    }); });
    it('should convert $id and $ownerId to identifiers inside where condition', function () { return __awaiter(void 0, void 0, void 0, function () {
        var id, ownerId;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    id = generateRandomIdentifier_1.default();
                    ownerId = generateRandomIdentifier_1.default();
                    return [4 /*yield*/, get_1.default.call(platform, 'app.withByteArrays', {
                            where: [
                                ['$id', '==', id.toString()],
                                ['$ownerId', '==', ownerId.toString()],
                            ],
                        })];
                case 1:
                    _a.sent();
                    chai_1.expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
                        appDefinition.contractId,
                        'withByteArrays',
                        {
                            where: [
                                ['$id', '==', id],
                                ['$ownerId', '==', ownerId],
                            ],
                        },
                    ]);
                    return [2 /*return*/];
            }
        });
    }); });
    it('should convert Document to identifiers inside where condition for "startAt" and "startAfter"', function () { return __awaiter(void 0, void 0, void 0, function () {
        var _a, docA, docB;
        return __generator(this, function (_b) {
            switch (_b.label) {
                case 0:
                    _a = getDocumentsFixture_1.default(), docA = _a[0], docB = _a[1];
                    return [4 /*yield*/, get_1.default.call(platform, 'app.withByteArrays', {
                            startAt: docA,
                            startAfter: docB,
                        })];
                case 1:
                    _b.sent();
                    chai_1.expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
                        appDefinition.contractId,
                        'withByteArrays',
                        {
                            startAt: docA.getId(),
                            startAfter: docB.getId(),
                        },
                    ]);
                    return [2 /*return*/];
            }
        });
    }); });
    it('should convert string to identifiers inside where condition for "startAt" and "startAfter"', function () { return __awaiter(void 0, void 0, void 0, function () {
        var _a, docA, docB;
        return __generator(this, function (_b) {
            switch (_b.label) {
                case 0:
                    _a = getDocumentsFixture_1.default(), docA = _a[0], docB = _a[1];
                    return [4 /*yield*/, get_1.default.call(platform, 'app.withByteArrays', {
                            startAt: docA.getId().toString('base58'),
                            startAfter: docB.getId().toString('base58'),
                        })];
                case 1:
                    _b.sent();
                    chai_1.expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
                        appDefinition.contractId,
                        'withByteArrays',
                        {
                            startAt: docA.getId(),
                            startAfter: docB.getId(),
                        },
                    ]);
                    return [2 /*return*/];
            }
        });
    }); });
    it('should convert nested identifier properties inside where condition if `elementMatch` is used', function () { return __awaiter(void 0, void 0, void 0, function () {
        var id;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    id = generateRandomIdentifier_1.default();
                    dataContract = getDataContractFixture_1.default();
                    dataContract.documents.withByteArrays.properties.nestedObject = {
                        type: 'object',
                        properties: {
                            idField: {
                                type: 'array',
                                byteArray: true,
                                contentMediaType: 'application/x.dash.dpp.identifier',
                                minItems: 32,
                                maxItems: 32,
                            },
                            anotherNested: {
                                type: 'object',
                                properties: {
                                    anotherIdField: {
                                        type: 'array',
                                        byteArray: true,
                                        contentMediaType: 'application/x.dash.dpp.identifier',
                                        minItems: 32,
                                        maxItems: 32,
                                    },
                                },
                            },
                        },
                    };
                    appDefinition = {
                        contractId: dataContract.getId(),
                        contract: dataContract,
                    };
                    appsGetMock.reset();
                    appsGetMock.returns(appDefinition);
                    return [4 /*yield*/, get_1.default.call(platform, 'app.withByteArrays', {
                            where: [
                                ['nestedObject', 'elementMatch', ['idField', '==', id.toString()]],
                                ['nestedObject', 'elementMatch', ['anotherNested', 'elementMatch', ['anotherIdField', '==', id.toString()]]],
                            ],
                        })];
                case 1:
                    _a.sent();
                    chai_1.expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
                        appDefinition.contractId,
                        'withByteArrays',
                        {
                            where: [
                                ['nestedObject', 'elementMatch', ['idField', '==', id]],
                                ['nestedObject', 'elementMatch', ['anotherNested', 'elementMatch', ['anotherIdField', '==', id]]],
                            ],
                        },
                    ]);
                    return [2 /*return*/];
            }
        });
    }); });
});
//# sourceMappingURL=get.spec.js.map