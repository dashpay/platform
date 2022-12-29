const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateDataContractUpdateTransitionStateFactory', () => {
  let validateDataContractUpdateTransitionState;
  let dataContract;
  let stateTransition;
  let executionContext;
  let DataContractUpdateTransition;
  let StateTransitionExecutionContext;
  let DataContractFactory;
  let DataContractValidator;
  let validateTransitionWithExistingContract;
  let DataContractNotPresentError;
  let InvalidDataContractVersionError;
  let ValidationResult;

  before(async () => {
    ({
      DataContractUpdateTransition,
      StateTransitionExecutionContext,
      validateDataContractUpdateTransitionState,
      ValidationResult,
      DataContractFactory,
      DataContractValidator,
      DataContractNotPresentError,
      InvalidDataContractVersionError,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = getDataContractFixture();

    const updatedRawDataContract = dataContract.toObject();

    updatedRawDataContract.version += 1;

    stateTransition = new DataContractUpdateTransition({
      dataContract: updatedRawDataContract,
      protocolVersion: protocolVersion.latestVersion,
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    const validator = new DataContractValidator();
    const dataContractFactory = new DataContractFactory(protocolVersion.latestVersion, validator);
    const wasmDataContract = await dataContractFactory.createFromBuffer(dataContract.toBuffer());

    const stateRepositoryLike = {
      fetchDataContract: () => wasmDataContract,
    };

    validateTransitionWithExistingContract = (t) => validateDataContractUpdateTransitionState(
      stateRepositoryLike,
      t,
    );
  });

  it('should return invalid result if Data Contract with specified contractId was not found', async () => {
    const stateRepositoryLikeNoDataContract = {
      fetchDataContract: () => undefined,
    };

    const validateTransitionWithNoContract = (t) => validateDataContractUpdateTransitionState(
      stateRepositoryLikeNoDataContract,
      t,
    );

    const result = await validateTransitionWithNoContract(stateTransition);

    expect(result.isValid()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataContractNotPresentError);
    expect(error.getCode()).to.equal(1018);
    expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());
  });

  it('should return invalid result if Data Contract version is not larger by 1', async () => {
    const badlyUpdatedRawDataContract = dataContract.toObject();
    badlyUpdatedRawDataContract.version += 2;

    const badStateTransition = new DataContractUpdateTransition({
      dataContract: badlyUpdatedRawDataContract,
      protocolVersion: protocolVersion.latestVersion,
    });

    const result = await validateTransitionWithExistingContract(badStateTransition);

    expect(result.isValid()).to.be.false();

    const [error] = result.getErrors();
    expect(error).to.be.an.instanceOf(InvalidDataContractVersionError);
    expect(error.getCode()).to.equal(1050);
  });

  it('should return valid result', async () => {
    const result = await validateTransitionWithExistingContract(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result on dry run', async () => {
    const stateRepositoryLikeNoDataContract = {
      fetchDataContract: () => undefined,
    };

    const validateTransitionWithNoContract = (t) => validateDataContractUpdateTransitionState(
      stateRepositoryLikeNoDataContract,
      t,
    );

    executionContext.enableDryRun();

    const result = await validateTransitionWithNoContract(stateTransition);

    executionContext.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
