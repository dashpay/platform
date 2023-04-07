"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
var chai_1 = require("chai");
var index_1 = __importDefault(require("./index"));
require("mocha");
describe('Dash', function () {
    it('should provide expected class', function () {
        chai_1.expect(index_1.default).to.have.property('Client');
        chai_1.expect(index_1.default.Client.name).to.be.equal('Client');
        chai_1.expect(index_1.default.Client.constructor.name).to.be.equal('Function');
    });
});
//# sourceMappingURL=SDK.spec.js.map