"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.ClientApps = void 0;
var Identifier_1 = __importDefault(require("@dashevo/dpp/lib/Identifier"));
var ClientApps = /** @class */ (function () {
    function ClientApps(apps) {
        var _this = this;
        if (apps === void 0) { apps = {}; }
        this.apps = {};
        Object.entries(apps).forEach(function (_a) {
            var name = _a[0], definition = _a[1];
            return _this.set(name, definition);
        });
    }
    /**
       * Set app
       *
       * @param {string} name
       * @param {ClientAppDefinitionOptions} definition
       */
    ClientApps.prototype.set = function (name, definition) {
        definition.contractId = Identifier_1.default.from(definition.contractId);
        this.apps[name] = definition;
    };
    /**
       * Get app definition by name
       *
       * @param {string} name
       * @return {ClientAppDefinition}
       */
    ClientApps.prototype.get = function (name) {
        if (!this.has(name)) {
            throw new Error("Application with name " + name + " is not defined");
        }
        return this.apps[name];
    };
    /**
       * Check if app is defined
       *
       * @param {string} name
       * @return {boolean}
       */
    ClientApps.prototype.has = function (name) {
        return Boolean(this.apps[name]);
    };
    /**
       * Get all apps
       *
       * @return {ClientAppsList}
       */
    ClientApps.prototype.getNames = function () {
        return Object.keys(this.apps);
    };
    return ClientApps;
}());
exports.ClientApps = ClientApps;
//# sourceMappingURL=ClientApps.js.map