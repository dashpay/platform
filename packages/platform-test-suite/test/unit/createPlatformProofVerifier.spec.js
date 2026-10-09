const { expect, use } = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

const createPlatformProofVerifier = require('../../lib/test/createPlatformProofVerifier');

use(chaiAsPromised);
use(sinonChai);

/**
 * Build a verifier over stubbed wait APIs.
 *
 * @param {Object} stateTransitions
 * @returns {{ verifier: Object, evo: Object, stateTransition: Object }}
 */
function createVerifierWith(stateTransitions) {
  const stateTransition = {};
  const evo = {
    StateTransition: {
      fromBytes: sinon.stub().returns(stateTransition),
    },
  };
  const verifier = createPlatformProofVerifier({
    getEvoSdkForNetwork: sinon.stub().resolves({ evo, sdk: { stateTransitions } }),
  });

  return { verifier, evo, stateTransition };
}

/**
 * The error the WASM SDK raises for a transition family that has no proof
 * binding one specific transition.
 *
 * @returns {Error}
 */
function executionNotProvedError() {
  const error = new Error('received a verified snapshot for this transition family');
  error.name = 'ExecutionNotProved';
  return error;
}

describe('createPlatformProofVerifier', () => {
  describe('quorum publication delay', () => {
    let clock;

    beforeEach(() => {
      clock = sinon.useFakeTimers();
    });

    afterEach(() => {
      clock.restore();
    });

    function missingQuorumError() {
      // Match the prototype getters and nested error reported by the WASM SDK in CI.
      return Object.create({
        name: 'DapiClientError',
        message: 'no available addresses to retry, last error: Proof verification error: '
          + 'context provider error: invalid quorum: Quorum not found in cache for hash: '
          + '0b32385bfc1e0147b7dee87de5db332105dfd603fef6070bd9c6323aa2493a46',
      });
    }

    const input = { serializedStateTransition: Uint8Array.from([1, 2, 3]), network: 'local' };

    it('should verify again after the sidecar publishes a rotated quorum', async () => {
      const waitForResponse = sinon.stub().callsFake(() => {
        if (waitForResponse.callCount === 1) {
          return Promise.reject(missingQuorumError());
        }
        if (Date.now() < 60000) {
          return Promise.reject(new Error('no available addresses'));
        }
        return Promise.resolve();
      });
      const waitForAffectedState = sinon.stub().resolves();
      const { verifier, stateTransition } = createVerifierWith({
        waitForResponse, waitForAffectedState,
      });
      const result = verifier.verifyStateTransitionResult(input).then(() => null, (error) => error);

      await clock.tickAsync(65000);

      expect(await result).to.equal(null);
      expect(clock.now).to.equal(65000);
      expect(waitForResponse.callCount).to.equal(2);
      expect(waitForResponse).to.have.always.been.calledWithExactly(stateTransition);
      expect(waitForAffectedState).to.not.have.been.called;
    });

    it('should fail after the bounded wait if the quorum never becomes available', async () => {
      const waitForResponse = sinon.stub().rejects(missingQuorumError());
      const waitForAffectedState = sinon.stub().resolves();
      const { verifier } = createVerifierWith({ waitForResponse, waitForAffectedState });
      const result = verifier.verifyStateTransitionResult(input).then(() => null, (error) => error);

      await clock.tickAsync(65000);

      expect(await result).to.be.an.instanceOf(Error)
        .with.property('message').that.includes('Quorum not found in cache');
      expect(waitForResponse.callCount).to.equal(2);
      expect(waitForAffectedState).to.not.have.been.called;
      expect(clock.countTimers()).to.equal(0);
    });

    it('should verify the affected-state proof again if its quorum is not published yet', async () => {
      const waitForResponse = sinon.stub().rejects(executionNotProvedError());
      const waitForAffectedState = sinon.stub().resolves();
      waitForAffectedState.onFirstCall().rejects(missingQuorumError());
      const { verifier } = createVerifierWith({ waitForResponse, waitForAffectedState });
      const result = verifier.verifyStateTransitionResult(input).then(() => null, (error) => error);

      await clock.tickAsync(65000);

      expect(await result).to.equal(null);
      expect(waitForAffectedState).to.have.been.calledTwice;
    });

    it('should stop immediately if the refreshed quorum reveals an invalid signature', async () => {
      const proofError = new Error('quorum signature is invalid');
      const waitForResponse = sinon.stub().rejects(proofError);
      waitForResponse.onFirstCall().rejects(missingQuorumError());
      const waitForAffectedState = sinon.stub().resolves();
      const { verifier } = createVerifierWith({ waitForResponse, waitForAffectedState });
      const result = verifier.verifyStateTransitionResult(input).then(() => null, (error) => error);

      await clock.tickAsync(65000);

      expect(await result).to.equal(proofError);
      expect(waitForResponse).to.have.been.calledTwice;
      expect(waitForAffectedState).to.not.have.been.called;
      expect(clock.countTimers()).to.equal(0);
    });
  });

  it('should require an execution proof before accepting a state transition', async () => {
    const waitForResponse = sinon.stub().resolves();
    const waitForAffectedState = sinon.stub().resolves();
    const { verifier, evo, stateTransition } = createVerifierWith({
      waitForResponse,
      waitForAffectedState,
    });
    const serializedStateTransition = Uint8Array.from([1, 2, 3]);

    await verifier.verifyStateTransitionResult({
      serializedStateTransition,
      network: 'local',
    });

    expect(evo.StateTransition.fromBytes).to.have.been.calledOnceWithExactly(
      serializedStateTransition,
    );
    expect(waitForResponse).to.have.been.calledOnceWithExactly(stateTransition);
    expect(waitForAffectedState).to.not.have.been.called;
  });

  it('should accept an affected-state snapshot only when execution cannot be proved', async () => {
    const waitForResponse = sinon.stub().rejects(executionNotProvedError());
    const waitForAffectedState = sinon.stub().resolves();
    const { verifier, stateTransition } = createVerifierWith({
      waitForResponse,
      waitForAffectedState,
    });

    await verifier.verifyStateTransitionResult({
      serializedStateTransition: Uint8Array.from([1, 2, 3]),
      network: 'local',
    });

    expect(waitForResponse).to.have.been.calledOnceWithExactly(stateTransition);
    expect(waitForAffectedState).to.have.been.calledOnceWithExactly(stateTransition);
  });

  it('should propagate a failed execution proof without falling back to a snapshot', async () => {
    const proofError = new Error('invalid execution proof');
    const waitForResponse = sinon.stub().rejects(proofError);
    const waitForAffectedState = sinon.stub().resolves();
    const { verifier } = createVerifierWith({ waitForResponse, waitForAffectedState });

    await expect(verifier.verifyStateTransitionResult({
      serializedStateTransition: Uint8Array.from([1, 2, 3]),
      network: 'local',
    })).to.be.rejectedWith(proofError);

    expect(waitForAffectedState).to.not.have.been.called;
  });

  it('should report a WASM error as a plain Error so its message survives', async () => {
    // A wasm-bindgen error object: kind and message live on the prototype, and
    // it is not an Error, so a parallel mocha worker serializes away everything
    // it carries.
    const wasmError = Object.create({
      get name() {
        return 'Proof';
      },
      get message() {
        return 'quorum signature is invalid';
      },
    });
    const { verifier } = createVerifierWith({
      waitForResponse: sinon.stub().rejects(wasmError),
      waitForAffectedState: sinon.stub().resolves(),
    });

    const error = await verifier.verifyStateTransitionResult({
      serializedStateTransition: Uint8Array.from([1, 2, 3]),
      network: 'local',
    }).then(() => null, (thrown) => thrown);

    expect(error).to.be.an.instanceOf(Error);
    expect(error.name).to.equal('Proof');
    expect(error.message).to.equal('Proof: quorum signature is invalid');
    expect(Object.getOwnPropertyNames(error)).to.include('message');
  });

  it('should propagate affected-state proof rejection', async () => {
    const proofError = new Error('invalid affected-state proof');
    const { verifier } = createVerifierWith({
      waitForResponse: sinon.stub().rejects(executionNotProvedError()),
      waitForAffectedState: sinon.stub().rejects(proofError),
    });

    await expect(verifier.verifyStateTransitionResult({
      serializedStateTransition: Uint8Array.from([1, 2, 3]),
      network: 'local',
    })).to.be.rejectedWith(proofError);
  });
});
