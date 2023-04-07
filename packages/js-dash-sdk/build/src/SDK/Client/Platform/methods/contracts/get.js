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
exports.get = void 0;
// @ts-ignore
var wasm_dpp_1 = __importDefault(require("@dashevo/wasm-dpp"));
var NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');
var Identifier;
var Metadata;
/**
 * Get contracts from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @returns contracts
 */
function get(identifier) {
    return __awaiter(this, void 0, void 0, function () {
        var contractId, _i, _a, appName, appDefinition, dataContractResponse, e_1, contract, metadata, responseMetadata, _b, _c, appName, appDefinition;
        var _d;
        return __generator(this, function (_e) {
            switch (_e.label) {
                case 0:
                    this.logger.debug("[Contracts#get] Get Data Contract \"" + identifier + "\"");
                    return [4 /*yield*/, this.initialize()];
                case 1:
                    _e.sent();
                    return [4 /*yield*/, wasm_dpp_1.default()];
                case 2:
                    // TODO(wasm): expose Metadata from dedicated module that handles all WASM-DPP types
                    (_d = _e.sent(), Metadata = _d.Metadata, Identifier = _d.Identifier);
                    contractId = Identifier.from(identifier);
                    // Try to get contract from the cache
                    // eslint-disable-next-line
                    for (_i = 0, _a = this.client.getApps().getNames(); _i < _a.length; _i++) {
                        appName = _a[_i];
                        appDefinition = this.client.getApps().get(appName);
                        if (appDefinition.contractId.equals(contractId) && appDefinition.contract) {
                            return [2 /*return*/, appDefinition.contract];
                        }
                    }
                    _e.label = 3;
                case 3:
                    _e.trys.push([3, 5, , 6]);
                    return [4 /*yield*/, this.client.getDAPIClient()
                            .platform.getDataContract(contractId)];
                case 4:
                    dataContractResponse = _e.sent();
                    this.logger.silly("[Contracts#get] Fetched Data Contract \"" + identifier + "\"");
                    return [3 /*break*/, 6];
                case 5:
                    e_1 = _e.sent();
                    if (e_1 instanceof NotFoundError) {
                        return [2 /*return*/, null];
                    }
                    throw e_1;
                case 6: return [4 /*yield*/, this.wasmDpp.dataContract
                        .createFromBuffer(dataContractResponse.getDataContract())];
                case 7:
                    contract = _e.sent();
                    metadata = null;
                    responseMetadata = dataContractResponse.getMetadata();
                    if (responseMetadata) {
                        metadata = new Metadata({
                            blockHeight: responseMetadata.getHeight(),
                            coreChainLockedHeight: responseMetadata.getCoreChainLockedHeight(),
                            timeMs: responseMetadata.getTimeMs(),
                            protocolVersion: responseMetadata.getProtocolVersion(),
                        });
                    }
                    contract.setMetadata(metadata);
                    // Store contract to the cache
                    // eslint-disable-next-line
                    for (_b = 0, _c = this.client.getApps().getNames(); _b < _c.length; _b++) {
                        appName = _c[_b];
                        appDefinition = this.client.getApps().get(appName);
                        if (appDefinition.contractId.equals(contractId)) {
                            appDefinition.contract = contract;
                        }
                    }
                    this.logger.debug("[Contracts#get] Obtained Data Contract \"" + identifier + "\"");
                    return [2 /*return*/, contract];
            }
        });
    });
}
exports.get = get;
exports.default = get;
//# sourceMappingURL=get.js.map