const validateSTPacketDPContractsFactory = require('../../../../lib/stPacket/validation/validateSTPacketDPContractsFactory');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');

const InvalidSTPacketContractIdError = require('../../../../lib/errors/InvalidSTPacketContractIdError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketDPContractsFactory', () => {
  let rawSTPacket;
  let rawDPContract;
  let dpContract;
  let validateSTPacketDPContracts;
  let validateDPContractMock;
  let createDPContractMock;

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

    createDPContractMock = this.sinonSandbox.stub().returns(dpContract);
    validateDPContractMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketDPContracts = validateSTPacketDPContractsFactory(
      validateDPContractMock,
      createDPContractMock,
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

    expect(createDPContractMock).to.be.not.called();
  });

  it('should return invalid result if STPacket\'s contractId is not equal to DP Contract ID', () => {
    rawSTPacket.contractId = 'wrong';

    const result = validateSTPacketDPContracts(rawSTPacket);

    expectValidationError(result, InvalidSTPacketContractIdError);

    expect(createDPContractMock).to.be.calledOnceWith(rawDPContract);

    const [error] = result.getErrors();

    expect(error.getDPContractId()).to.be.equal(rawSTPacket.contractId);
    expect(error.getDPContract()).to.be.equal(dpContract);
  });

  it('should return valid result if DP Contract is valid and STPacket\'s contractId is correct', () => {
    createDPContractMock.returns(dpContract);

    const result = validateSTPacketDPContracts(rawSTPacket);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(createDPContractMock).to.be.calledOnceWith(rawDPContract);
  });
});
