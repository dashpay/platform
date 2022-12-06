const calculateStateTransitionFee = require('@dashevo/dpp/lib/stateTransition/fee/calculateStateTransitionFee');

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/PreCalculatedOperation');
const DummyFeeResult = require('@dashevo/dpp/lib/stateTransition/fee/DummyFeeResult');

describe('calculateStateTransitionFee', () => {
  let stateTransition;

  beforeEach(async () => {
    const privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';

    stateTransition = getIdentityCreateTransitionFixture();
    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);
  });

  // TODO: Must be more comprehensive. After we settle all factors and formula.
  it('should calculate fee based on executed operations', () => {
    const executionContext = stateTransition.getExecutionContext();

    executionContext.addOperation(
      new ReadOperation(10),
      new PreCalculatedOperation(new DummyFeeResult(12, 12)),
    );

    const result = calculateStateTransitionFee(stateTransition);

    expect(result).to.equal(17088);
  });
});
