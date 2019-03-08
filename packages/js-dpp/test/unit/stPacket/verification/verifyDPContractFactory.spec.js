const verifyDPContractFactory = require('../../../../lib/stPacket/verification/verifyDPContractFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

const DPContractAlreadyPresentError = require('../../../../lib/errors/DPContractAlreadyPresentError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

describe('verifyDPContract', () => {
  let verifyDPContract;
  let dataProviderMock;
  let dpContract;
  let stPacket;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    verifyDPContract = verifyDPContractFactory(dataProviderMock);

    dpContract = getDPContractFixture();

    stPacket = new STPacket(dpContract.getId());
    stPacket.setDPContract(dpContract);
  });

  it('should return invalid result if DP Contract is already present', async () => {
    dataProviderMock.fetchDPContract.resolves(dpContract);

    const result = await verifyDPContract(stPacket);

    expectValidationError(result, DPContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDPContract()).to.equal(dpContract);

    expect(dataProviderMock.fetchDPContract).to.have.been.calledOnceWith(dpContract.getId());
  });

  it('should return valid result if DP Contract is not present', async () => {
    const result = await verifyDPContract(stPacket);

    expectValidationError(result, DPContractAlreadyPresentError, 0);
  });
});
