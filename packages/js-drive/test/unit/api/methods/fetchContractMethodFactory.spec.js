const fetchContractMethodFactory = require('../../../../lib/api/methods/fetchContractMethodFactory');

const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');

const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');

describe('fetchContractMethodFactory', () => {
  let contract;
  let contractId;
  let fetchContractMethod;
  let fetchContractMock;

  beforeEach(function beforeEach() {
    contract = getContractFixture();
    contractId = contract.getId();

    fetchContractMock = this.sinon.stub();
    fetchContractMethod = fetchContractMethodFactory(fetchContractMock);
  });

  it('should throw InvalidParamsError if Contract ID is not provided', async () => {
    let error;
    try {
      await fetchContractMethod({});
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidParamsError);

    expect(fetchContractMock).to.have.not.been.called();
  });

  it('should throw error if Contract is not found', async () => {
    fetchContractMock.returns(null);

    let error;
    try {
      await fetchContractMethod({ contractId });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidParamsError);

    expect(fetchContractMock).to.have.been.calledOnceWith(contractId);
  });

  it('should return DP contract', async () => {
    fetchContractMock.returns(contract);

    const result = await fetchContractMethod({ contractId });

    expect(result).to.deep.equal(contract.toJSON());

    expect(fetchContractMock).to.have.been.calledOnceWith(contractId);
  });
});
