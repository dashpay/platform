import { SinonSandbox } from 'sinon';

export function createDapiClientMock(sinon: SinonSandbox) {
  return {
    platform: {
      broadcastStateTransition: sinon.stub(),
      getIdentity: sinon.stub(),
      waitForStateTransitionResult: sinon.stub().resolves({}),
      getDataContract: sinon.stub(),
    },
  };
}
