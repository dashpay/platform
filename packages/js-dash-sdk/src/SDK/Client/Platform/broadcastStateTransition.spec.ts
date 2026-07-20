import { expect } from 'chai';
import broadcastStateTransition from './broadcastStateTransition';

describe('broadcastStateTransition proof requirements', function suite() {
  it('fails before broadcasting when no proof verifier is configured', async function test() {
    const dapi = {
      platform: {
        broadcastStateTransition: this.sinon.stub(),
        waitForStateTransitionResult: this.sinon.stub(),
      },
    };
    const platform = {
      client: {
        getPlatformProofVerifier: () => undefined,
        getDAPIClient: () => dapi,
      },
      initialize: this.sinon.stub(),
    };

    await expect(broadcastStateTransition(
      platform as any,
      { toBuffer: () => Buffer.from('transition') },
    )).to.be.rejectedWith('requires an authenticated Platform proof verifier');

    expect(dapi.platform.broadcastStateTransition).not.to.have.been.called;
    expect(platform.initialize).not.to.have.been.called;
  });

  it('rejects a success response missing proof material', async function test() {
    const verifier = {
      verifyStateTransitionResult: this.sinon.stub(),
    };
    const dapi = {
      platform: {
        broadcastStateTransition: this.sinon.stub().resolves(),
        waitForStateTransitionResult: this.sinon.stub().resolves({ metadata: {} }),
      },
    };
    const platform = {
      client: {
        network: 'testnet',
        getPlatformProofVerifier: () => verifier,
        getDAPIClient: () => dapi,
      },
      initialize: this.sinon.stub().resolves(),
      protocolVersion: 13,
    };

    await expect(broadcastStateTransition(
      platform as any,
      { toBuffer: () => Buffer.from('transition') },
    )).to.be.rejectedWith('missing proof or metadata');

    expect(verifier.verifyStateTransitionResult).not.to.have.been.called;
  });
});
