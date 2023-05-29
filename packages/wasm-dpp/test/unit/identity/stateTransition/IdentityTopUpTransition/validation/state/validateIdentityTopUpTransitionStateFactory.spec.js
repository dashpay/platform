const getIdentityTopUpTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateIdentityTopUpTransitionStateFactory', () => {
  let validateIdentityTopUpTransitionState;
  let stateTransition;
  let stateRepositoryMock;

  let IdentityTopUpTransitionStateValidator;

  before(async () => {
    ({
      IdentityTopUpTransitionStateValidator,
    } = await loadWasmDpp());
  });

  beforeEach(async function () {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = await getIdentityTopUpTransitionFixture();

    const validator = new IdentityTopUpTransitionStateValidator(stateRepositoryMock);

    validateIdentityTopUpTransitionState = (st) => validator.validate(
      st,
    );
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
