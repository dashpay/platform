const getIdentityTopUpTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../../../../../../dist');

describe.skip('validateIdentityTopUpTransitionStateFactory', () => {
  let validateIdentityTopUpTransitionState;
  let stateTransition;
  let stateRepositoryMock;
  let executionContext;

  let IdentityTopUpTransitionStateValidator;
  let StateTransitionExecutionContext;

  before(async () => {
    ({
      IdentityTopUpTransitionStateValidator,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    stateTransition = await getIdentityTopUpTransitionFixture();

    const validator = new IdentityTopUpTransitionStateValidator(stateRepositoryMock);

    executionContext = new StateTransitionExecutionContext();
    validateIdentityTopUpTransitionState = (st) => validator.validate(st, executionContext);
  });

  it('should return valid result', async () => {
    const result = await validateIdentityTopUpTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();
  });
});
