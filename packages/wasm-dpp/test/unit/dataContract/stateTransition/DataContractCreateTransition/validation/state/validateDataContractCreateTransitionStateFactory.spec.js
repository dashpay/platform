const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../../../../..');
const { getLatestProtocolVersion } = require('../../../../../../..');

describe.skip('validateDataContractCreateTransitionStateFactory', () => {
  let validateDataContractCreateTransitionState;
  let dataContract;
  let stateTransition;
  let executionContext;
  let ValidationResult;
  let DataContractCreateTransition;
  let StateTransitionExecutionContext;
  let factory;
  let dataContractFetched;
  let DataContractFactory;
  let DataContractValidator;

  before(async () => {
    ({
      DataContractCreateTransition,
      StateTransitionExecutionContext,
      validateDataContractCreateTransitionState,
      ValidationResult,
      DataContractFactory,
      DataContractValidator,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();
    stateTransition = new DataContractCreateTransition({
      protocolVersion: getLatestProtocolVersion(),
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    executionContext = new StateTransitionExecutionContext();

    dataContractFetched = false;

    const stateRepositoryLike = {
      fetchDataContract: async () => {
        dataContractFetched = true;
      },
    };

    factory = (t) => validateDataContractCreateTransitionState(
      stateRepositoryLike,
      t,
      executionContext,
    );
  });

  it('should return invalid result if Data Contract is already exist', async () => {
    // This time our state repository should return a data contract on fetch
    const validator = new DataContractValidator();
    const dataContractFactory = new DataContractFactory(1, validator);
    const reCreatedDataContract = await dataContractFactory
      .createFromBuffer(dataContract.toBuffer());

    const stateRepositoryLikeWithContract = {
      fetchDataContract: async () => reCreatedDataContract,
    };

    const result = await validateDataContractCreateTransitionState(
      stateRepositoryLikeWithContract,
      stateTransition,
      executionContext,
    );
    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4000);
    expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());
  });

  it('should return valid result', async () => {
    const result = await factory(stateTransition);

    expect(dataContractFetched).to.be.true();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result on dry run', async () => {
    executionContext.enableDryRun();
    const result = await factory(stateTransition);
    executionContext.disableDryRun();

    expect(dataContractFetched).to.be.true();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
