const getIdentityTopUpTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityTopUpTransitionFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateIdentityTopUpTransitionStateFactory', () => {
  let validateIdentityTopUpTransitionState;
  let stateTransition;
  let stateRepositoryMock;

  let IdentityTopUpTransition;

  let validateIdentityTopUpTransitionStateDPP;

  before(async () => {
    ({
      IdentityTopUpTransition,
      validateIdentityTopUpTransitionState: validateIdentityTopUpTransitionStateDPP,
    } = await loadWasmDpp());
  });

  beforeEach(function () {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = new IdentityTopUpTransition(
      getIdentityTopUpTransitionFixture().toObject(),
    );

    validateIdentityTopUpTransitionState = (st) => validateIdentityTopUpTransitionStateDPP(
      stateRepositoryMock, st,
    );
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
