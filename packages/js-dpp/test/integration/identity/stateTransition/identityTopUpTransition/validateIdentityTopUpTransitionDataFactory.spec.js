const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const validateIdentityTopUpTransitionDataFactory = require(
  '../../../../../lib/identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionDataFactory',
);

describe('validateIdentityTopUpTransitionDataFactory', () => {
  let validateIdentityTopUpTransitionData;
  let stateTransition;

  beforeEach(() => {
    validateIdentityTopUpTransitionData = validateIdentityTopUpTransitionDataFactory();

    stateTransition = getIdentityTopUpTransitionFixture();
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionData(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
