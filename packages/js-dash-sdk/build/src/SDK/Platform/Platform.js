"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = exports.Platform = void 0;
// @ts-ignore
var dpp_1 = __importDefault(require("@dashevo/dpp"));
var Platform_1 = require("../Client/Platform/Platform");
var Platform;
(function (Platform) {
    Platform.DashPlatformProtocol = dpp_1.default;
    Platform.initializeDppModule = Platform_1.Platform.initializeDppModule;
})(Platform = exports.Platform || (exports.Platform = {}));
exports.default = Platform;
//# sourceMappingURL=Platform.js.map