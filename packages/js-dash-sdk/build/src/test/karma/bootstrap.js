"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
var sinon_1 = __importDefault(require("sinon"));
var chai_1 = require("chai");
var dirty_chai_1 = __importDefault(require("dirty-chai"));
var sinon_chai_1 = __importDefault(require("sinon-chai"));
chai_1.use(dirty_chai_1.default);
chai_1.use(sinon_chai_1.default);
beforeEach(function beforeEach() {
    if (!this.sinon) {
        this.sinon = sinon_1.default.createSandbox();
    }
    else {
        this.sinon.restore();
    }
});
afterEach(function afterEach() {
    this.sinon.restore();
});
//# sourceMappingURL=bootstrap.js.map