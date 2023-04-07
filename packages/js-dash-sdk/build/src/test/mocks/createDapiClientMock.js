"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createDapiClientMock = void 0;
function createDapiClientMock(sinon) {
    return {
        platform: {
            broadcastStateTransition: sinon.stub(),
            getIdentity: sinon.stub(),
            waitForStateTransitionResult: sinon.stub().resolves({}),
            getDataContract: sinon.stub(),
        },
    };
}
exports.createDapiClientMock = createDapiClientMock;
//# sourceMappingURL=createDapiClientMock.js.map