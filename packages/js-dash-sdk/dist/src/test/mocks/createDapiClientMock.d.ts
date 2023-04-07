import { SinonSandbox } from 'sinon';
export declare function createDapiClientMock(sinon: SinonSandbox): {
    platform: {
        broadcastStateTransition: import("sinon").SinonStub<any[], any>;
        getIdentity: import("sinon").SinonStub<any[], any>;
        waitForStateTransitionResult: import("sinon").SinonStub<any[], any>;
        getDataContract: import("sinon").SinonStub<any[], any>;
    };
};
