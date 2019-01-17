const validateSTPacketDPContractsFactory = require('../../../../lib/stPacket/validation/validateSTPacketDPContractsFactory');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketDPContractsFactory', () => {
  let rawSTPacket;
  let rawDPContract;
  let dpContract;
  let validateSTPacketDPContracts;
  let validateDPContractMock;

  beforeEach(function beforeEach() {
    dpContract = getDPContractFixture();
    rawDPContract = dpContract.toJSON();
    rawSTPacket = {
      contractId: dpContract.getId(),
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [rawDPContract],
      objects: [],
    };

    validateDPContractMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketDPContracts = validateSTPacketDPContractsFactory(
      validateDPContractMock,
    );
  });

  it('should return invalid result if DP Contract is wrong', () => {
    const dpContractError = new ConsensusError('test');

    validateDPContractMock.returns(
      new ValidationResult([dpContractError]),
    );

    const result = validateSTPacketDPContracts(rawSTPacket);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.equal(dpContractError);
  });

  it('should return valid result if DP Contract is valid', () => {
    const result = validateSTPacketDPContracts(rawSTPacket);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
