const { expectValidationError } = require(
  '@dashevo/dpp/lib/test/expect/expectError',
);

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const { default: loadWasmDpp } = require('../../../../../../../dist');
const generateRandomIdentifierAsync = require('../../../../../../../lib/test/utils/generateRandomIdentifierAsync');

describe('validateIdentityCreateTransitionStateFactory', () => {
  let IdentityCreateTransition;
  let StateTransitionExecutionContext;
  let IdentityAlreadyExistsError;
  let Identity;
  let ValidationResult;

  let validateIdentityCreateTransitionState;
  let validate;
  let executionContext;
  let rawIdentity;

  let stateTransition;
  let stateRepositoryMock;

  before(async () => {
    ({
      IdentityCreateTransition,
      StateTransitionExecutionContext,
      ValidationResult,
      IdentityAlreadyExistsError,
      Identity,
      validateIdentityCreateTransitionState,
    } = await loadWasmDpp());
  });

  beforeEach(async function () {
    const stFixture = getIdentityCreateTransitionFixture();
    stateTransition = new IdentityCreateTransition(stFixture.toObject());

    executionContext = new StateTransitionExecutionContext();
    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    rawIdentity = {
      protocolVersion: protocolVersion.latestVersion,
      id: await generateRandomIdentifierAsync(),
      publicKeys: [
        {
          id: 0,
          type: 0,
          data: Buffer.alloc(36).fill('a'),
          purpose: 0,
          securityLevel: 0,
          readOnly: false,
          signature: Buffer.alloc(36).fill('a'),
        },
      ],
      balance: 0,
      revision: 0,
    };

    validate = (st) => validateIdentityCreateTransitionState(stateRepositoryMock, st);
  });

  it('should return invalid result if identity already exists', async () => {
    const identity = new Identity(rawIdentity);

    stateRepositoryMock.fetchIdentity.returns(identity);

    const result = await validate(stateTransition);

    expectValidationError(result, IdentityAlreadyExistsError, 1, ValidationResult);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4011);
    expect(error.getIdentityId()).to.exist();
    expect(error.getIdentityId()).to.deep.equal(stateTransition.getIdentityId().toBuffer());
  });

  it('should return valid result if state transition is valid', async () => {
    const result = await validate(stateTransition);

    expect(result.isValid()).to.be.true();
  });

  it('should return valid result on dry run', async () => {
    executionContext.enableDryRun();

    const result = await validate(stateTransition);

    executionContext.disableDryRun();
    expect(stateRepositoryMock.fetchIdentity).to.have.been.called();

    expect(result.isValid()).to.be.true();
  });
});
