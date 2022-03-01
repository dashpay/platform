const calculateStateTransitionFee = require('../../../lib/stateTransition/calculateStateTransitionFee');

const getIdentityCreateTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

describe('calculateStateTransitionFee', () => {
  let stateTransition;
  let stateTransitionSize;

  beforeEach(async () => {
    const privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';

    stateTransition = getIdentityCreateTransitionFixture();
    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    stateTransitionSize = stateTransition.toBuffer({ skipSignature: true }).length;
  });

  it('should calculate fee based on state transition size', () => {
    const result = calculateStateTransitionFee(stateTransition);

    expect(result).to.equal(stateTransitionSize * calculateStateTransitionFee.PRICE_PER_BYTE);
  });
});
