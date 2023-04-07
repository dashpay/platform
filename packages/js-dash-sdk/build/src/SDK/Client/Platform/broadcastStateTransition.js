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
var crypto_1 = __importDefault(require("crypto"));
var StateTransitionBroadcastError_1 = require("../../../errors/StateTransitionBroadcastError");
var ResponseError = require('@dashevo/dapi-client/lib/transport/errors/response/ResponseError');
var InvalidRequestDPPError = require('@dashevo/dapi-client/lib/transport/errors/response/InvalidRequestDPPError');
var createGrpcTransportError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/createGrpcTransportError');
var GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
/**
 * @param {Platform} platform
 * @param {Object} [options]
 * @param {boolean} [options.skipValidation=false]
 *
 * @param stateTransition
 */
function broadcastStateTransition(platform, stateTransition, options) {
    if (options === void 0) { options = {}; }
    return __awaiter(this, void 0, void 0, function () {
        var client, wasmDpp, result, consensusError, hash, serializedStateTransition, error_1, cause, stateTransitionResult, error, grpcError, cause;
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    client = platform.client, wasmDpp = platform.wasmDpp;
                    if (!!options.skipValidation) return [3 /*break*/, 2];
                    return [4 /*yield*/, wasmDpp.stateTransition.validateBasic(stateTransition)];
                case 1:
                    result = _a.sent();
                    if (!result.isValid()) {
                        consensusError = result.getFirstError();
                        // TODO(wasm): make sure code, message and error are present
                        //  and that StateTransitionBroadcastError handles consensusError correctly
                        throw new StateTransitionBroadcastError_1.StateTransitionBroadcastError(consensusError.getCode(), consensusError.message, consensusError);
                    }
                    _a.label = 2;
                case 2:
                    hash = crypto_1.default.createHash('sha256')
                        .update(stateTransition.toBuffer())
                        .digest();
                    serializedStateTransition = stateTransition.toBuffer();
                    _a.label = 3;
                case 3:
                    _a.trys.push([3, 5, , 6]);
                    return [4 /*yield*/, client.getDAPIClient().platform.broadcastStateTransition(serializedStateTransition)];
                case 4:
                    _a.sent();
                    return [3 /*break*/, 6];
                case 5:
                    error_1 = _a.sent();
                    if (error_1 instanceof ResponseError) {
                        cause = error_1;
                        // Pass DPP consensus error directly to avoid
                        // additional wrappers
                        if (cause instanceof InvalidRequestDPPError) {
                            cause = cause.getConsensusError();
                        }
                        // TODO(wasm): make sure code, message and error are present
                        //  and that StateTransitionBroadcastError handles consensusError correctly
                        throw new StateTransitionBroadcastError_1.StateTransitionBroadcastError(cause.getCode(), cause.message, cause);
                    }
                    throw error_1;
                case 6: return [4 /*yield*/, client
                        .getDAPIClient().platform.waitForStateTransitionResult(hash, { prove: true })];
                case 7:
                    stateTransitionResult = _a.sent();
                    error = stateTransitionResult.error;
                    if (!error) return [3 /*break*/, 9];
                    grpcError = new GrpcError(error.code, error.message);
                    // It is important to assign metadata to the error object
                    // instead of passing it as GrpcError constructor argument
                    // Otherwise it will be converted to grpc-js metadata
                    // Which is not compatible with web
                    grpcError.metadata = error.data;
                    return [4 /*yield*/, createGrpcTransportError(grpcError)];
                case 8:
                    cause = _a.sent();
                    // Pass DPP consensus error directly to avoid
                    // additional wrappers
                    if (cause instanceof InvalidRequestDPPError) {
                        cause = cause.getConsensusError();
                    }
                    throw new StateTransitionBroadcastError_1.StateTransitionBroadcastError(cause.getCode(), cause.message, cause);
                case 9: return [2 /*return*/, stateTransitionResult];
            }
        });
    });
}
exports.default = broadcastStateTransition;
//# sourceMappingURL=broadcastStateTransition.js.map