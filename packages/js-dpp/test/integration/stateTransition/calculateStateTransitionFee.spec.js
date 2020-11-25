const calculateStateTransitionFee = require('../../../lib/stateTransition/calculateStateTransitionFee');

const getIdentityCreateTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

describe('calculateStateTransitionFee', () => {
  let stateTransition;
  let stateTransitionSize;

  beforeEach(() => {
    const privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';

    stateTransition = getIdentityCreateTransitionFixture();
    stateTransition.signByPrivateKey(privateKey);

    stateTransitionSize = stateTransition.toBuffer({ skipSignature: true }).length;
  });

  it('should calculate fee based on state transition size', () => {
    const result = calculateStateTransitionFee(stateTransition);

    expect(result).to.equal(stateTransitionSize * calculateStateTransitionFee.PRICE_PER_BYTE);
  });
});
