const { expect, use } = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

const createPlatformProofVerifier = require('../../lib/test/createPlatformProofVerifier');

use(chaiAsPromised);
use(sinonChai);

describe('createPlatformProofVerifier', () => {
  it('should verify state transitions through the affected-state proof API', async () => {
    const stateTransition = {};
    const evo = {
      StateTransition: {
        fromBytes: sinon.stub().returns(stateTransition),
      },
    };
    const waitForAffectedState = sinon.stub().resolves();
    const waitForResponse = sinon.stub().resolves();
    const verifier = createPlatformProofVerifier({
      getEvoSdkForNetwork: sinon.stub().resolves({
        evo,
        sdk: {
          stateTransitions: {
            waitForAffectedState,
            waitForResponse,
          },
        },
      }),
    });
    const serializedStateTransition = Uint8Array.from([1, 2, 3]);

    await verifier.verifyStateTransitionResult({
      serializedStateTransition,
      network: 'local',
    });

    expect(evo.StateTransition.fromBytes).to.have.been.calledOnceWithExactly(
      serializedStateTransition,
    );
    expect(waitForAffectedState).to.have.been.calledOnceWithExactly(stateTransition);
    expect(waitForResponse).not.to.have.been.called;
  });

  it('should propagate affected-state proof rejection', async () => {
    const proofError = new Error('invalid affected-state proof');
    const verifier = createPlatformProofVerifier({
      getEvoSdkForNetwork: sinon.stub().resolves({
        evo: {
          StateTransition: {
            fromBytes: sinon.stub().returns({}),
          },
        },
        sdk: {
          stateTransitions: {
            waitForAffectedState: sinon.stub().rejects(proofError),
          },
        },
      }),
    });

    await expect(verifier.verifyStateTransitionResult({
      serializedStateTransition: Uint8Array.from([1, 2, 3]),
      network: 'local',
    })).to.be.rejectedWith(proofError);
  });
});
