const applyIdentityUpdateTransitionFactory = require('../../../../../lib/identity/stateTransition/IdentityUpdateTransition/applyIdentityUpdateTransitionFactory');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const StateTransitionExecutionContext = require('../../../../../lib/stateTransition/StateTransitionExecutionContext');
const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

describe('applyIdentityUpdateTransition', () => {
  let applyIdentityUpdateTransition;
  let stateRepositoryMock;
  let stateTransition;
  let executionContext;

  beforeEach(function beforeEach() {
    stateTransition = getIdentityUpdateTransitionFixture();
    stateTransition.setRevision(stateTransition.getRevision() + 1);

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    applyIdentityUpdateTransition = applyIdentityUpdateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should add and disable public keys', async () => {
    await applyIdentityUpdateTransition(stateTransition);

    expect(stateRepositoryMock.updateIdentityRevision).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      stateTransition.getRevision(),
      executionContext,
    );

    expect(stateRepositoryMock.disableIdentityKeys).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      stateTransition.getPublicKeyIdsToDisable(),
      stateTransition.getPublicKeysDisabledAt().getTime(),
      executionContext,
    );

    const publicKeysToAdd = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => {
        const rawPublicKey = publicKey.toObject({ skipSignature: true });

        return new IdentityPublicKey(rawPublicKey);
      });

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      publicKeysToAdd,
      executionContext,
    );
  });

  it('should add and disable public keys on dry run', async () => {
    stateTransition.getExecutionContext().enableDryRun();

    await applyIdentityUpdateTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(stateRepositoryMock.updateIdentityRevision).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      stateTransition.getRevision(),
      executionContext,
    );

    expect(stateRepositoryMock.disableIdentityKeys).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      stateTransition.getPublicKeyIdsToDisable(),
      stateTransition.getPublicKeysDisabledAt().getTime(),
      executionContext,
    );

    const publicKeysToAdd = stateTransition.getPublicKeysToAdd()
      .map((publicKey) => {
        const rawPublicKey = publicKey.toObject({ skipSignature: true });

        return new IdentityPublicKey(rawPublicKey);
      });

    expect(stateRepositoryMock.addKeysToIdentity).to.be.calledOnceWithExactly(
      stateTransition.getOwnerId(),
      publicKeysToAdd,
      executionContext,
    );
  });
});
