const getIdentityTopUpTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityTopUpTransitionFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateIdentityTopUpTransitionStateFactory', () => {
  let validateIdentityTopUpTransitionState;
  let stateTransition;
  let stateRepositoryMock;
  let executionContext;

  let IdentityTopUpTransition;
  let IdentityTopUpTransitionStateValidator;
  let StateTransitionExecutionContext;

  before(async () => {
    ({
      IdentityTopUpTransition,
      IdentityTopUpTransitionStateValidator,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
  });

  beforeEach(function () {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = new IdentityTopUpTransition(
      getIdentityTopUpTransitionFixture().toObject(),
    );

    const validator = new IdentityTopUpTransitionStateValidator(stateRepositoryMock);

    executionContext = new StateTransitionExecutionContext();
    validateIdentityTopUpTransitionState = (st) => validator.validate(
      st, executionContext,
    );
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
