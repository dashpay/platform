const rewiremock = require('rewiremock/node');

const Document = require('../../../lib/document/Document');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../lib/test/fixtures/getContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const InvalidDocumentError = require('../../../lib/document/errors/InvalidDocumentError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DocumentFactory', () => {
  let hashMock;
  let decodeMock;
  let generateMock;
  let validateDocumentMock;
  let DocumentFactory;
  let userId;
  let contract;
  let document;
  let rawDocument;
  let factory;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    decodeMock = this.sinonSandbox.stub();
    generateMock = this.sinonSandbox.stub();
    validateDocumentMock = this.sinonSandbox.stub();

    DocumentFactory = rewiremock.proxy('../../../lib/document/DocumentFactory', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/util/entropy': { generate: generateMock },
      '../../../lib/document/Document': Document,
    });

    ({ userId } = getDocumentsFixture);
    contract = getContractFixture();

    [document] = getDocumentsFixture();
    rawDocument = document.toJSON();

    factory = new DocumentFactory(
      userId,
      contract,
      validateDocumentMock,
    );
  });

  describe('create', () => {
    it('should return new Document with specified type and data', () => {
      const scope = '123';
      const scopeId = '456';
      const name = 'Cutie';

      hashMock.returns(scope);
      generateMock.returns(scopeId);

      const newDocument = factory.create(
        rawDocument.$type,
        { name },
      );

      expect(newDocument).to.be.an.instanceOf(Document);

      expect(newDocument.getType()).to.equal(rawDocument.$type);

      expect(newDocument.get('name')).to.equal(name);

      expect(hashMock).to.have.been.calledOnceWith(contract.getId() + userId);
      expect(newDocument.scope).to.equal(scope);

      expect(generateMock).to.have.been.calledOnce();
      expect(newDocument.scopeId).to.equal(scopeId);

      expect(newDocument.getAction()).to.equal(Document.DEFAULTS.ACTION);

      expect(newDocument.getRevision()).to.equal(Document.DEFAULTS.REVISION);
    });

    it('should throw an error if type is not defined', () => {
      const type = 'wrong';

      let error;
      try {
        factory.create(type);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
      expect(error.getType()).to.equal(type);
      expect(error.getContract()).to.equal(contract);

      expect(hashMock).to.have.not.been.called();
    });
  });

  describe('createFromObject', () => {
    it('should return new Contract with data from passed object', () => {
      validateDocumentMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDocument);

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toJSON()).to.deep.equal(rawDocument);

      expect(validateDocumentMock).to.have.been.calledOnceWith(rawDocument, contract);
    });

    it('should return new Document without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawDocument, { skipValidation: true });

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toJSON()).to.deep.equal(rawDocument);

      expect(validateDocumentMock).to.have.not.been.called();
    });

    it('should throw an error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDocumentMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDocument);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawDocument()).to.equal(rawDocument);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.equal(validationError);

      expect(validateDocumentMock).to.have.been.calledOnceWith(rawDocument, contract);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Contract from serialized Contract', () => {
      const serializedDocument = document.serialize();

      decodeMock.returns(rawDocument);

      factory.createFromObject.returns(document);

      const result = factory.createFromSerialized(serializedDocument);

      expect(result).to.equal(document);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDocument);

      expect(decodeMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });

  describe('setUserId', () => {
    it('should set User ID', () => {
      userId = '123';

      const result = factory.setUserId(userId);

      expect(result).to.equal(factory);
      expect(factory.userId).to.equal(userId);
    });
  });

  describe('getUserId', () => {
    it('should return User ID', () => {
      const result = factory.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('setContract', () => {
    it('should set Contract', () => {
      factory.contract = null;

      const result = factory.setContract(contract);

      expect(result).to.equal(factory);
      expect(factory.contract).to.equal(contract);
    });
  });

  describe('getContract', () => {
    it('should return Contract', () => {
      const result = factory.getContract();

      expect(result).to.equal(contract);
    });
  });
});
