"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
var Identifier_1 = __importDefault(require("@dashevo/dpp/lib/Identifier"));
var chai_1 = require("chai");
var ClientApps_1 = require("./ClientApps");
require("mocha");
describe('ClientApps', function () {
    var apps;
    it('constructor', function () {
        apps = new ClientApps_1.ClientApps();
        chai_1.expect(apps.apps).to.deep.equal({});
    });
    it('.set', function () {
        apps.set('dpns', {
            contractId: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
            contract: { someField: true },
        });
        apps.set('tutorialContract', {
            contractId: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
            contract: { someField: true },
        });
    });
    it('should get', function () {
        var getByName = apps.get('dpns');
        chai_1.expect(getByName).to.deep.equal({
            contractId: Identifier_1.default.from('3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8'),
            contract: { someField: true },
        });
    });
    it('should .getNames()', function () {
        var names = apps.getNames();
        chai_1.expect(names).to.deep.equal(['dpns', 'tutorialContract']);
    });
    it('should .has', function () {
        chai_1.expect(apps.has('dpns')).to.equal(true);
        chai_1.expect(apps.has('tutorialContract')).to.equal(true);
        chai_1.expect(apps.has('tutorialContractt')).to.equal(false);
    });
});
//# sourceMappingURL=ClientApps.spec.js.map