const calculateStateTransitionFee = require('../../../lib/stateTransition/fee/calculateStateTransitionFee');

const getIdentityCreateTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const ReadOperation = require('../../../lib/stateTransition/fee/operations/ReadOperation');
const WriteOperation = require('../../../lib/stateTransition/fee/operations/WriteOperation');
const DeleteOperation = require('../../../lib/stateTransition/fee/operations/DeleteOperation');
const PreCalculatedOperation = require('../../../lib/stateTransition/fee/operations/PreCalculatedOperation');

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
      new WriteOperation(5, 5),
      new DeleteOperation(6, 6),
      new PreCalculatedOperation(12, 12),
    );

    const result = calculateStateTransitionFee(stateTransition);

    expect(result).to.equal(13616);
  });
});
