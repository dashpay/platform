"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = exports.SDK = void 0;
var dapi_client_1 = __importDefault(require("@dashevo/dapi-client"));
var wallet_lib_1 = require("@dashevo/wallet-lib");
var Client_1 = require("./Client");
var Core_1 = require("./Core");
var Platform_1 = require("./Platform");
var StateTransitionBroadcastError_1 = require("../errors/StateTransitionBroadcastError");
var SDK;
(function (SDK) {
    SDK.DAPIClient = dapi_client_1.default;
    SDK.Client = Client_1.Client;
    SDK.Core = Core_1.Core;
    // TODO: consider marking as DEPRECATED and use PlatformProtocol below instead
    SDK.Platform = Platform_1.Platform;
    // Wallet-lib primitives
    SDK.Wallet = wallet_lib_1.Wallet;
    SDK.Account = wallet_lib_1.Account;
    SDK.KeyChain = wallet_lib_1.DerivableKeyChain;
    // TODO: consider merging into Wallet above and mark as DEPRECATED
    SDK.WalletLib = {
        CONSTANTS: wallet_lib_1.CONSTANTS,
        EVENTS: wallet_lib_1.EVENTS,
        plugins: wallet_lib_1.plugins,
        utils: wallet_lib_1.utils,
    };
    SDK.PlatformProtocol = SDK.Platform.DashPlatformProtocol;
    SDK.Essentials = {
        Buffer: Buffer,
    };
    SDK.Errors = {
        StateTransitionBroadcastError: StateTransitionBroadcastError_1.StateTransitionBroadcastError,
    };
})(SDK = exports.SDK || (exports.SDK = {}));
exports.default = SDK;
//# sourceMappingURL=SDK.js.map