const rewiremock = require('rewiremock/node');

const STPacketHeader = require('../../../lib/stPacketHeader/STPacketHeader');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidSTPacketHeaderError = require('../../../lib/stPacket/errors/InvalidSTPacketHeaderError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('STPacketHeaderFactory', () => {
  let decodeMock;
  let STPacketHeaderFactory;
  let validateSTPacketHeaderMock;
  let factory;
  let contractId;
  let stPacketHeader;
  let rawSTPacketHeader;
  let itemsMerkleRoot;
  let itemsHash;

  beforeEach(function beforeEach() {
    decodeMock = this.sinonSandbox.stub();
    validateSTPacketHeaderMock = this.sinonSandbox.stub();

    // Require STPacketHeaderFactory for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/stPacketHeader/STPacketHeaderFactory');

    STPacketHeaderFactory = rewiremock.proxy('../../../lib/stPacketHeader/STPacketHeaderFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/stPacketHeader/STPacketHeader': STPacketHeader,
    });

    contractId = '5586b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b74';
    itemsMerkleRoot = '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b';
    itemsHash = 'y90b273ff34fce19d6b804eff5a3f5747ada4eaa22f86fj5jf652ddb78755642';

    factory = new STPacketHeaderFactory(validateSTPacketHeaderMock);

    stPacketHeader = new STPacketHeader(contractId, itemsMerkleRoot, itemsHash);

    rawSTPacketHeader = stPacketHeader.toJSON();
  });

  describe('create', () => {
    it('should return new STPacketHeader', () => {
      const newSTPacketHeader = factory.create(
        contractId,
        itemsMerkleRoot,
        itemsHash,
      );

      expect(newSTPacketHeader).to.be.an.instanceOf(STPacketHeader);

      expect(newSTPacketHeader.getContractId()).to.equal(contractId);
      expect(newSTPacketHeader.getItemsMerkleRoot()).to.equal(itemsMerkleRoot);
      expect(newSTPacketHeader.getItemsHash()).to.equal(itemsHash);
    });
  });

  describe('createFromObject', () => {
    it('should return new STPacketHeader with data from passed object', () => {
      validateSTPacketHeaderMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawSTPacketHeader);

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.toJSON()).to.deep.equal(rawSTPacketHeader);

      expect(validateSTPacketHeaderMock).to.have.been.calledOnceWith(rawSTPacketHeader);
    });

    it('should throw an error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateSTPacketHeaderMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawSTPacketHeader);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidSTPacketHeaderError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawSTPacketHeader()).to.equal(rawSTPacketHeader);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.equal(validationError);

      expect(validateSTPacketHeaderMock).to.have.been.calledOnceWith(rawSTPacketHeader);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Contract from serialized Contract', () => {
      const serializedSTPacket = stPacketHeader.serialize();

      decodeMock.returns(rawSTPacketHeader);

      factory.createFromObject.returns(stPacketHeader);

      const result = factory.createFromSerialized(serializedSTPacket);

      expect(result).to.equal(stPacketHeader);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawSTPacketHeader);

      expect(decodeMock).to.have.been.calledOnceWith(serializedSTPacket);
    });
  });
});
