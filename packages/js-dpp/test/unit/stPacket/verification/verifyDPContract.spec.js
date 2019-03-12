const verifyDPContract = require('../../../../lib/stPacket/verification/verifyDPContract');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const DPContractAlreadyPresentError = require('../../../../lib/errors/DPContractAlreadyPresentError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

describe('verifyDPContract', () => {
  let dpContract;
  let stPacket;

  beforeEach(() => {
    dpContract = getDPContractFixture();

    stPacket = new STPacket(dpContract.getId());
    stPacket.setDPContract(dpContract);
  });

  it('should return invalid result if DP Contract is already present', async () => {
    const result = await verifyDPContract(stPacket, dpContract);

    expectValidationError(result, DPContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDPContract()).to.equal(dpContract);
  });

  it('should return valid result if DP Contract is not present', async () => {
    const result = await verifyDPContract(stPacket, undefined);

    expectValidationError(result, DPContractAlreadyPresentError, 0);
  });
});
