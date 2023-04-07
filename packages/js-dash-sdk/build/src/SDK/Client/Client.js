"use strict";
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
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
exports.Client = void 0;
var events_1 = require("events");
var wallet_lib_1 = require("@dashevo/wallet-lib");
var DAPIClientTransport_1 = __importDefault(require("@dashevo/wallet-lib/src/transport/DAPIClientTransport/DAPIClientTransport"));
var dapi_client_1 = __importDefault(require("@dashevo/dapi-client"));
var systemIds_1 = require("@dashevo/dpns-contract/lib/systemIds");
var systemIds_2 = require("@dashevo/dashpay-contract/lib/systemIds");
var systemIds_3 = require("@dashevo/masternode-reward-shares-contract/lib/systemIds");
var Platform_1 = require("./Platform");
var ClientApps_1 = require("./ClientApps");
/**
 * Client class that wraps all components together
 * to allow integrated payments on both the Dash Network (layer 1)
 * and the Dash Platform (layer 2).
 */
var Client = /** @class */ (function (_super) {
    __extends(Client, _super);
    /**
       * Construct some instance of SDK Client
       *
       * @param {ClientOpts} [options] - options for SDK Client
       */
    function Client(options) {
        if (options === void 0) { options = {}; }
        var _a;
        var _this = _super.call(this) || this;
        _this.network = 'testnet';
        _this.defaultAccountIndex = 0;
        _this.options = options;
        _this.network = _this.options.network ? _this.options.network.toString() : 'testnet';
        // Initialize DAPI Client
        var dapiClientOptions = {
            network: _this.network,
        };
        [
            'dapiAddressProvider',
            'dapiAddresses',
            'seeds',
            'timeout',
            'retries',
            'baseBanTime',
            'blockHeadersProviderOptions',
            'blockHeadersProvider',
        ].forEach(function (optionName) {
            // eslint-disable-next-line
            if (_this.options.hasOwnProperty(optionName)) {
                dapiClientOptions[optionName] = _this.options[optionName];
            }
        });
        _this.dapiClient = new dapi_client_1.default(dapiClientOptions);
        // Initialize a wallet if `wallet` option is preset
        if (_this.options.wallet !== undefined) {
            if (_this.options.wallet.network !== undefined
                && _this.options.wallet.network !== _this.network) {
                throw new Error('Wallet and Client networks are different');
            }
            var transport = new DAPIClientTransport_1.default(_this.dapiClient);
            _this.wallet = new wallet_lib_1.Wallet(__assign({ transport: transport, network: _this.network }, _this.options.wallet));
            // @ts-ignore
            _this.wallet.on('error', function (error, context) { return (_this.emit('error', error, { wallet: context })); });
        }
        // @ts-ignore
        _this.defaultAccountIndex = ((_a = _this.options.wallet) === null || _a === void 0 ? void 0 : _a.defaultAccountIndex) || 0;
        _this.apps = new ClientApps_1.ClientApps(__assign({ dpns: {
                contractId: systemIds_1.contractId,
            }, dashpay: {
                contractId: systemIds_2.contractId,
            }, masternodeRewardShares: {
                contractId: systemIds_3.contractId,
            } }, _this.options.apps));
        _this.platform = new Platform_1.Platform({
            client: _this,
            network: _this.network,
            driveProtocolVersion: _this.options.driveProtocolVersion,
        });
        return _this;
    }
    /**
       * Get Wallet account
       *
       * @param {Account.Options} [options]
       * @returns {Promise<Account>}
       */
    Client.prototype.getWalletAccount = function (options) {
        if (options === void 0) { options = {}; }
        return __awaiter(this, void 0, void 0, function () {
            return __generator(this, function (_a) {
                if (!this.wallet) {
                    throw new Error('Wallet is not initialized, pass `wallet` option to Client');
                }
                options = __assign({ index: this.defaultAccountIndex }, options);
                return [2 /*return*/, this.wallet.getAccount(options)];
            });
        });
    };
    /**
       * disconnect wallet from Dapi
       * @returns {void}
       */
    Client.prototype.disconnect = function () {
        return __awaiter(this, void 0, void 0, function () {
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0:
                        if (!this.wallet) return [3 /*break*/, 2];
                        return [4 /*yield*/, this.wallet.disconnect()];
                    case 1:
                        _a.sent();
                        _a.label = 2;
                    case 2: return [2 /*return*/];
                }
            });
        });
    };
    /**
       * Get DAPI Client instance
       *
       * @returns {DAPIClient}
       */
    Client.prototype.getDAPIClient = function () {
        return this.dapiClient;
    };
    /**
       * fetch list of applications
       *
       * @remarks
       * check if returned value can be null on devnet
       *
       * @returns {ClientApps} applications list
       */
    Client.prototype.getApps = function () {
        return this.apps;
    };
    return Client;
}(events_1.EventEmitter));
exports.Client = Client;
exports.default = Client;
//# sourceMappingURL=Client.js.map