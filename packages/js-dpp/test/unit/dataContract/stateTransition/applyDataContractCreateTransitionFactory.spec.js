const DataContractCreateTransition = require(
  '../../../../lib/dataContract/stateTransition/DataContractCreateTransition',
);

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const applyDataContractCreateTransitionFactory = require(
  '../../../../lib/dataContract/stateTransition/applyDataContractCreateTransitionFactory',
);

const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

describe('applyDataContractCreateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let stateRepositoryMock;
  let applyDataContractCreateTransition;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: 'yMhYUcMgUgYZGwBnTEyWFmRhqQSjZv7twq',
    });

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    applyDataContractCreateTransition = applyDataContractCreateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should store a data contract from state transition in the repository', async () => {
    await applyDataContractCreateTransition(stateTransition);

    expect(stateRepositoryMock.storeDataContract).to.have.been.calledOnceWithExactly(
      stateTransition.getDataContract(),
    );
  });
});
