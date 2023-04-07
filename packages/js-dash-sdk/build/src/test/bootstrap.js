"use strict";
var dotenvSafe = require('dotenv-safe');
var path = require('path');
var sinon = require('sinon');
var sinonChai = require('sinon-chai');
var use = require('chai').use;
var dirtyChai = require('dirty-chai');
dotenvSafe.config({
    path: path.resolve(__dirname, '..', '..', '.env'),
});
use(dirtyChai);
use(sinonChai);
before(function before() {
    if (!this.sinon) {
        this.sinon = sinon.createSandbox();
    }
    else {
        this.sinon.restore();
    }
});
after(function after() {
    this.sinon.restore();
});
beforeEach(function beforeEach() {
    if (!this.sinon) {
        this.sinon = sinon.createSandbox();
    }
    else {
        this.sinon.restore();
    }
});
afterEach(function afterEach() {
    this.sinon.restore();
});
//# sourceMappingURL=bootstrap.js.map