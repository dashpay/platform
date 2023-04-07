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
var DataContractFactory_1 = __importDefault(require("@dashevo/dpp/lib/dataContract/DataContractFactory"));
var ValidationResult_1 = __importDefault(require("@dashevo/dpp/lib/validation/ValidationResult"));
var Identifier_1 = __importDefault(require("@dashevo/dpp/lib/Identifier"));
var getResponseMetadataFixture_1 = __importDefault(require("../../../../../test/fixtures/getResponseMetadataFixture"));
var get_1 = __importDefault(require("./get"));
var identities_json_1 = __importDefault(require("../../../../../../tests/fixtures/identities.json"));
var contracts_json_1 = __importDefault(require("../../../../../../tests/fixtures/contracts.json"));
require("mocha");
var ClientApps_1 = require("../../../ClientApps");
var GetDataContractResponse = require('@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse');
var NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');
var factory = new DataContractFactory_1.default(undefined, function () { return new ValidationResult_1.default(); }, function () { return [42, contracts_json_1.default.ratePlatform]; });
var dpp = {
    dataContract: factory,
    getProtocolVersion: function () { return 42; },
};
factory.dpp = dpp;
var apps = new ClientApps_1.ClientApps({
    ratePlatform: {
        contractId: contracts_json_1.default.ratePlatform.$id,
    },
});
var client;
var askedFromDapi;
var initialize;
var metadataFixture;
describe('Client - Platform - Contracts - .get()', function () {
    before(function before() {
        var _this = this;
        metadataFixture = getResponseMetadataFixture_1.default();
        askedFromDapi = 0;
        var getDataContract = function (id) { return __awaiter(_this, void 0, void 0, function () {
            var fixtureIdentifier, contract;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0:
                        fixtureIdentifier = Identifier_1.default.from(contracts_json_1.default.ratePlatform.$id);
                        askedFromDapi += 1;
                        if (!id.equals(fixtureIdentifier)) return [3 /*break*/, 2];
                        return [4 /*yield*/, dpp.dataContract.createFromObject(contracts_json_1.default.ratePlatform)];
                    case 1:
                        contract = _a.sent();
                        return [2 /*return*/, new GetDataContractResponse(contract.toBuffer(), metadataFixture)];
                    case 2: throw new NotFoundError();
                }
            });
        }); };
        client = {
            getDAPIClient: function () { return ({
                platform: {
                    getDataContract: getDataContract,
                },
            }); },
            getApps: function () {
                return apps;
            },
        };
        initialize = this.sinon.stub();
    });
    describe('get a contract from string', function () {
        it('should get from DAPIClient if there is none locally', function () { return __awaiter(void 0, void 0, void 0, function () {
            var contract;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, get_1.default.call({
                            // @ts-ignore
                            apps: apps, dpp: dpp, client: client, initialize: initialize,
                        }, contracts_json_1.default.ratePlatform.$id)];
                    case 1:
                        contract = _a.sent();
                        chai_1.expect(contract.toJSON()).to.deep.equal(contracts_json_1.default.ratePlatform);
                        chai_1.expect(contract.getMetadata().getBlockHeight()).to.equal(10);
                        chai_1.expect(contract.getMetadata().getCoreChainLockedHeight()).to.equal(42);
                        chai_1.expect(contract.getMetadata().getTimeMs()).to.equal(metadataFixture.getTimeMs());
                        chai_1.expect(contract.getMetadata().getProtocolVersion())
                            .to.equal(metadataFixture.getProtocolVersion());
                        chai_1.expect(askedFromDapi).to.equal(1);
                        return [2 /*return*/];
                }
            });
        }); });
        it('should get from local when already fetched once', function () { return __awaiter(void 0, void 0, void 0, function () {
            var contract;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, get_1.default.call({
                            // @ts-ignore
                            apps: apps, dpp: dpp, client: client, initialize: initialize,
                        }, contracts_json_1.default.ratePlatform.$id)];
                    case 1:
                        contract = _a.sent();
                        chai_1.expect(contract.toJSON()).to.deep.equal(contracts_json_1.default.ratePlatform);
                        chai_1.expect(contract.getMetadata().getBlockHeight()).to.equal(10);
                        chai_1.expect(contract.getMetadata().getCoreChainLockedHeight()).to.equal(42);
                        chai_1.expect(contract.getMetadata().getTimeMs()).to.equal(metadataFixture.getTimeMs());
                        chai_1.expect(contract.getMetadata().getProtocolVersion())
                            .to.equal(metadataFixture.getProtocolVersion());
                        chai_1.expect(askedFromDapi).to.equal(1);
                        return [2 /*return*/];
                }
            });
        }); });
    });
    describe('other conditions', function () {
        it('should deal when contract do not exist', function () { return __awaiter(void 0, void 0, void 0, function () {
            var contract;
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, get_1.default.call({
                            // @ts-ignore
                            apps: apps, dpp: dpp, client: client, initialize: initialize,
                        }, identities_json_1.default.bob.id)];
                    case 1:
                        contract = _a.sent();
                        chai_1.expect(contract).to.equal(null);
                        return [2 /*return*/];
                }
            });
        }); });
    });
});
//# sourceMappingURL=get.spec.js.map