"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
var chai_1 = require("chai");
var protocolVersion_1 = require("@dashevo/dpp/lib/version/protocolVersion");
var index_1 = require("./index");
require("mocha");
var Client_1 = __importDefault(require("../Client"));
describe('Dash - Platform', function () {
    it('should provide expected class', function () {
        chai_1.expect(index_1.Platform.name).to.be.equal('Platform');
        chai_1.expect(index_1.Platform.constructor.name).to.be.equal('Function');
    });
    it('should set protocol version for DPP though options', function () {
        var platform = new index_1.Platform({
            client: new Client_1.default(),
            network: 'testnet',
            driveProtocolVersion: 42,
        });
        chai_1.expect(platform.dpp.protocolVersion).to.equal(42);
    });
    it('should set protocol version for DPP using mapping', function () {
        var platform = new index_1.Platform({
            client: new Client_1.default(),
            network: 'testnet',
        });
        // @ts-ignore
        var testnetProtocolVersion = index_1.Platform.networkToProtocolVersion.get('testnet');
        chai_1.expect(platform.dpp.protocolVersion).to.equal(testnetProtocolVersion);
    });
    it('should set protocol version for DPP using latest version', function () {
        var platform = new index_1.Platform({
            client: new Client_1.default(),
            network: 'unknown',
        });
        chai_1.expect(platform.dpp.protocolVersion).to.equal(protocolVersion_1.latestVersion);
    });
});
//# sourceMappingURL=Platform.spec.js.map