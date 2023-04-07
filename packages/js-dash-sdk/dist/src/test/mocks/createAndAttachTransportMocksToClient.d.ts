/// <reference types="sinon" />
export declare function createAndAttachTransportMocksToClient(client: any, sinon: any): Promise<{
    txStreamMock: any;
    transportMock: any;
    dapiClientMock: {
        platform: {
            broadcastStateTransition: import("sinon").SinonStub<any[], any>;
            getIdentity: import("sinon").SinonStub<any[], any>;
            waitForStateTransitionResult: import("sinon").SinonStub<any[], any>;
            getDataContract: import("sinon").SinonStub<any[], any>;
        };
    };
}>;
