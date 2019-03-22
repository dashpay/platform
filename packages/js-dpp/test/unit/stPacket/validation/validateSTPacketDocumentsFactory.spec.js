const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const validateSTPacketDocumentsFactory = require('../../../../lib/stPacket/validation/validateSTPacketDocumentsFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const DuplicateDocumentsError = require('../../../../lib/errors/DuplicateDocumentsError');
const InvalidContractError = require('../../../../lib/errors/InvalidContractError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateSTPacketDocumentsFactory', () => {
  let rawSTPacket;
  let contract;
  let rawDocuments;
  let findDuplicateDocumentsMock;
  let findDuplicateDocumentsByIndicesMock;
  let validateDocumentMock;
  let validateSTPacketDocuments;

  beforeEach(function beforeEach() {
    contract = getContractFixture();
    rawDocuments = getDocumentsFixture().map(o => o.toJSON());
    rawSTPacket = {
      contractId: contract.getId(),
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
      documents: rawDocuments,
    };

    findDuplicateDocumentsMock = this.sinonSandbox.stub().returns([]);
    findDuplicateDocumentsByIndicesMock = this.sinonSandbox.stub().returns([]);
    validateDocumentMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateSTPacketDocuments = validateSTPacketDocumentsFactory(
      validateDocumentMock,
      findDuplicateDocumentsMock,
      findDuplicateDocumentsByIndicesMock,
    );
  });

  it('should return invalid result if ST Packet has different ID than Contract', () => {
    rawSTPacket.contractId = '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b';

    const result = validateSTPacketDocuments(rawSTPacket, contract);

    expectValidationError(result, InvalidContractError);

    const [error] = result.getErrors();

    expect(error.getContract()).to.equal(contract);
    expect(error.getRawSTPacket()).to.equal(rawSTPacket);

    expect(validateDocumentMock.callCount).to.equal(5);

    rawSTPacket.documents.forEach((rawDocument) => {
      expect(validateDocumentMock).to.have.been.calledWith(rawDocument, contract);
    });
  });

  it('should return invalid result if there are duplicates Documents', () => {
    findDuplicateDocumentsMock.returns([rawDocuments[0]]);

    const result = validateSTPacketDocuments(rawSTPacket, contract);

    expectValidationError(result, DuplicateDocumentsError);

    expect(findDuplicateDocumentsMock).to.have.been.calledOnceWith(rawDocuments);

    const [error] = result.getErrors();

    expect(error.getDuplicatedDocuments()).to.deep.equal([rawDocuments[0]]);

    expect(validateDocumentMock.callCount).to.equal(5);

    rawSTPacket.documents.forEach((rawDocument) => {
      expect(validateDocumentMock).to.have.been.calledWith(rawDocument, contract);
    });
  });

  it('should return invalid result if Documents are invalid', () => {
    const documentError = new ConsensusError('test');

    validateDocumentMock.onCall(0).returns(
      new ValidationResult([documentError]),
    );

    const result = validateSTPacketDocuments(rawSTPacket, contract);

    expectValidationError(result, ConsensusError, 1);

    expect(findDuplicateDocumentsMock).to.have.been.calledOnceWith(rawDocuments);

    expect(validateDocumentMock.callCount).to.equal(5);

    const [error] = result.getErrors();

    expect(error).to.equal(documentError);
  });

  it('should return valid result if there are no duplicate Documents and they are valid', () => {
    const result = validateSTPacketDocuments(rawSTPacket, contract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(findDuplicateDocumentsMock).to.have.been.calledOnceWith(rawDocuments);

    expect(validateDocumentMock.callCount).to.equal(5);
  });
});
