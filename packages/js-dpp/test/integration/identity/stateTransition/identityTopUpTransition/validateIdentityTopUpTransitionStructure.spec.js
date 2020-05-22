const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const validateIdentityTopUpTransitionStructure = require(
  '../../../../../lib/identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionStructure',
);

const IdentityTopUpTransition = require(
  '../../../../../lib/identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition',
);

describe('validateIdentityTopUpTransitionStructure', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(() => {
    stateTransition = getIdentityTopUpTransitionFixture();

    rawStateTransition = stateTransition.toJSON();
  });

  it('should pass valid raw state transition', () => {
    const result = validateIdentityTopUpTransitionStructure(rawStateTransition);

    expect(result.isValid()).to.be.true();
  });

  it('should pass valid state transition', () => {
    const result = validateIdentityTopUpTransitionStructure(
      new IdentityTopUpTransition(rawStateTransition),
    );

    expect(result.isValid()).to.be.true();
  });
});
