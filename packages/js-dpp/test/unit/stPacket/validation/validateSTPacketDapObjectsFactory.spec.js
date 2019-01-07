const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');
const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const validateSTPacketDPObjectsFactory = require('../../../../lib/stPacket/validation/validateSTPacketDPObjectsFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const DuplicatedDPObjectsError = require('../../../../lib/errors/DuplicatedDPObjectsError');
const InvalidDPContractError = require('../../../../lib/errors/InvalidDPContractError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketDPObjectsFactory', () => {
  let rawSTPacket;
  let dpContract;
  let rawDPObjects;
  let findDuplicatedDPObjectsMock;
  let validateDPObjectMock;
  let validateSTPacketDPObjects;

  beforeEach(function beforeEach() {
    dpContract = getDPContractFixture();
    rawDPObjects = getDPObjectsFixture().map(o => o.toJSON());
    rawSTPacket = {
      contractId: dpContract.getId(),
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
      objects: rawDPObjects,
    };

    findDuplicatedDPObjectsMock = this.sinonSandbox.stub().returns([]);
    validateDPObjectMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketDPObjects = validateSTPacketDPObjectsFactory(
      validateDPObjectMock,
      findDuplicatedDPObjectsMock,
    );
  });

  it('should return invalid result if ST Packet has different ID than DPContract', () => {
    rawSTPacket.contractId = '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b';

    const result = validateSTPacketDPObjects(rawSTPacket, dpContract);

    expectValidationError(result, InvalidDPContractError);

    const [error] = result.getErrors();

    expect(error.getDPContract()).to.be.equal(dpContract);
    expect(error.getRawSTPacket()).to.be.equal(rawSTPacket);

    expect(validateDPObjectMock).to.be.calledThrice();

    rawSTPacket.objects.forEach((rawDPObject) => {
      expect(validateDPObjectMock).to.be.calledWith(rawDPObject, dpContract);
    });
  });

  it('should return invalid result if there are duplicates DP Objects', () => {
    findDuplicatedDPObjectsMock.returns([rawDPObjects[0]]);

    const result = validateSTPacketDPObjects(rawSTPacket, dpContract);

    expectValidationError(result, DuplicatedDPObjectsError);

    expect(findDuplicatedDPObjectsMock).to.be.calledOnceWith(rawDPObjects);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDPObjects()).to.be.deep.equal([rawDPObjects[0]]);

    expect(validateDPObjectMock).to.be.calledThrice();

    rawSTPacket.objects.forEach((rawDPObject) => {
      expect(validateDPObjectMock).to.be.calledWith(rawDPObject, dpContract);
    });
  });

  it('should return invalid result if DP Objects are invalid', () => {
    const dpObjectError = new ConsensusError('test');

    validateDPObjectMock.onCall(0).returns(
      new ValidationResult([dpObjectError]),
    );

    const result = validateSTPacketDPObjects(rawSTPacket, dpContract);

    expectValidationError(result, ConsensusError, 1);

    expect(findDuplicatedDPObjectsMock).to.be.calledOnceWith(rawDPObjects);

    expect(validateDPObjectMock).to.be.calledThrice();

    const [error] = result.getErrors();

    expect(error).to.be.equal(dpObjectError);
  });

  it('should return valid result if there are no duplicate DP Objects and they are valid', () => {
    const result = validateSTPacketDPObjects(rawSTPacket, dpContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(findDuplicatedDPObjectsMock).to.be.calledOnceWith(rawDPObjects);

    expect(validateDPObjectMock).to.be.calledThrice();
  });
});
