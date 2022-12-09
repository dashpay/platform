const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validateDataContractCreateTransitionStateFactory', () => {
  let validateDataContractCreateTransitionState;
  let dataContract;
  let stateTransition;
  let executionContext;
  let ValidationResult;
  let fetchDataContract;
  let DataContractCreateTransition;
  let ApplyDataContractCreateTransition;
  let StateTransitionExecutionContext;
  let factory;

  before(async () => {
    ({
      DataContractCreateTransition, StateTransitionExecutionContext, validateDataContractCreateTransitionState, ValidationResult
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
      }
    };

    factory = (stateTransition) => { return validateDataContractCreateTransitionState(stateRepositoryLike, stateTransition); };
  });

  // TODO wait for error types
  // it('should return invalid result if Data Contract with specified contractId is already exist', async () => {
  //   stateRepositoryMock.fetchDataContract.resolves(dataContract);

  //   const result = await validateDataContractCreateTransitionState(stateTransition);

  //   expectValidationError(result, DataContractAlreadyPresentError);

  //   const [error] = result.getErrors();

  //   expect(error.getCode()).to.equal(4000);
  //   expect(Buffer.isBuffer(error.getDataContractId())).to.be.true();
  //   expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

  //   expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
  //     dataContract.getId(),
  //     executionContext,
  //   );
  // });

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
