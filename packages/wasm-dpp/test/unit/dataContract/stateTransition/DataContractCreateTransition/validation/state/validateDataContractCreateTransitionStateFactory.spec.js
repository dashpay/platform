const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateDataContractCreateTransitionStateFactory', () => {
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

  beforeEach(() => {
    dataContract = getDataContractFixture();
    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    dataContractFetched = false;

    const stateRepositoryLike = {
      fetchDataContract: () => {
        dataContractFetched = true;
      },
    };

    factory = (t) => validateDataContractCreateTransitionState(stateRepositoryLike, t);
  });

  it('should return invalid result if Data Contract is already exist', async () => {
    // This time our state repository should return a data contract on fetch
    const validator = new DataContractValidator();
    const dataContractFactory = new DataContractFactory(1, validator);
    const wasmDataContract  = await dataContractFactory.createFromBuffer(dataContract.toBuffer());

    const stateRepositoryLikeWithContract = {
      fetchDataContract: () => {
        return wasmDataContract;
      }
    };

    const result = await validateDataContractCreateTransitionState(
      stateRepositoryLikeWithContract,
      stateTransition,
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
