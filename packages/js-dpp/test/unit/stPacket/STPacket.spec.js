const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const DataContract = require('../../../lib/dataContract/DataContract');
const Document = require('../../../lib/document/Document');

const DataContractAndDocumentsNotAllowedSamePacketError = require('../../../lib/stPacket/errors/DataContractAndDocumentsNotAllowedSamePacketError');

describe.skip('STPacket', () => {
  let hashMock;
  let encodeMock;
  let STPacket;
  let dataContract;
  let documents;
  let stPacket;
  let itemsHash;
  let itemsMerkleRoot;
  let contractId;
  let calculateItemsMerkleRootMock;
  let calculateItemsHashMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture();

    contractId = dataContract.getId();
    itemsHash = '14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293';
    itemsMerkleRoot = '44207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc292';

    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;
    calculateItemsMerkleRootMock = this.sinonSandbox.stub().returns(itemsMerkleRoot);
    calculateItemsHashMock = this.sinonSandbox.stub().returns(itemsHash);

    STPacket = rewiremock.proxy('../../../lib/stPacket/STPacket', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
      '../../../lib/contract/DataContract': DataContract,
      '../../../lib/document/Document': Document,
      '../../../lib/stPacket/calculateItemsMerkleRoot': calculateItemsMerkleRootMock,
      '../../../lib/stPacket/calculateItemsHash': calculateItemsHashMock,
    });

    stPacket = new STPacket(contractId);
  });

  describe('constructor', () => {
    it('should return new ST Packet with specified DataContract ID', () => {
      expect(stPacket).to.be.an.instanceOf(STPacket);

      expect(stPacket.contractId).to.equal(contractId);
    });

    it('should return new STPacket with specified DataContract ID and DataContract', () => {
      const result = new STPacket(contractId, dataContract);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.getContractId()).to.equal(contractId);
      expect(result.getContract()).to.equal(dataContract);
    });

    it('should return new STPacket with specified DataContract ID and Documents', () => {
      const result = new STPacket(contractId, documents);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.getContractId()).to.equal(contractId);
      expect(result.getDocuments()).to.equal(documents);
    });
  });

  describe('#setContractId', () => {
    it('should set DataContract ID', () => {
      const result = stPacket.setContractId(contractId);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.contractId).to.equal(contractId);
    });
  });

  describe('#getContractId', () => {
    it('should return DataContract ID', () => {
      const result = stPacket.getContractId();

      expect(result).to.equal(contractId);
    });
  });

  describe('#getItemsMerkleRoot', () => {
    it('should return items merkle root', () => {
      const result = stPacket.getItemsMerkleRoot();

      expect(result).to.equal(itemsMerkleRoot);

      expect(calculateItemsMerkleRootMock).to.have.been.calledOnceWith({
        contracts: stPacket.contracts,
        documents: stPacket.documents,
      });
    });
  });

  describe('#getItemsHash', () => {
    it('should return items hash', () => {
      const result = stPacket.getItemsHash();

      expect(result).to.equal(itemsHash);

      expect(calculateItemsHashMock).to.have.been.calledOnceWith({
        contracts: stPacket.contracts,
        documents: stPacket.documents,
      });
    });
  });

  describe('#setDataContract', () => {
    it('should set DataContract', () => {
      const result = stPacket.setContract(dataContract);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(stPacket.contracts).to.have.lengthOf(1);
      expect(stPacket.contracts[0]).to.equal(dataContract);
    });

    it('should throw an error if Documents are present', () => {
      stPacket.setDocuments(documents);

      let error;
      try {
        stPacket.setContract(dataContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(DataContractAndDocumentsNotAllowedSamePacketError);

      expect(error.getSTPacket()).to.equal(stPacket);
    });
  });

  describe('#getDataContract', () => {
    it('should return DataContract', () => {
      stPacket.contracts = [dataContract];

      const result = stPacket.getContract();

      expect(result).to.equal(dataContract);
    });

    it('should return null of DataContract is not present', () => {
      const result = stPacket.getContract();

      expect(result).to.be.null();
    });
  });

  describe('#setDocuments', () => {
    it('should set Documents and replace previous', () => {
      stPacket.setDocuments([documents[0]]);

      const result = stPacket.setDocuments(documents);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(stPacket.documents).to.equal(documents);
    });

    it('should throw an error if DataContract is present', () => {
      stPacket.setContract(dataContract);

      let error;
      try {
        stPacket.setDocuments(documents);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(DataContractAndDocumentsNotAllowedSamePacketError);

      expect(error.getSTPacket()).to.equal(stPacket);
    });
  });

  describe('#getDocuments', () => {
    it('should return Documents', () => {
      stPacket.documents = documents;

      const result = stPacket.getDocuments();

      expect(result).to.equal(documents);
    });
  });

  describe('#addDocument', () => {
    it('should add Document', () => {
      stPacket.addDocument(documents[0]);

      const result = stPacket.addDocument(documents[1], documents[2], documents[3], documents[4]);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(stPacket.documents).to.deep.equal(documents);
    });
  });

  describe('#toJSON', () => {
    it('should return ST Packet as plain object', () => {
      hashMock.returns(itemsHash);

      stPacket.setContract(dataContract);

      const result = stPacket.toJSON();

      expect(result).to.deep.equal({
        contractId,
        itemsMerkleRoot,
        itemsHash,
        documents: [],
        contracts: [dataContract.toJSON()],
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized ST Packet', () => {
      stPacket.setContract(dataContract);

      const serializedSTPacket = '123';

      encodeMock.returns(serializedSTPacket);

      const result = stPacket.serialize();

      const rawDataContract = dataContract.toJSON();

      expect(result).to.equal(serializedSTPacket);

      expect(encodeMock).to.have.been.calledOnceWith({
        contractId,
        itemsMerkleRoot,
        itemsHash,
        documents: [],
        contracts: [rawDataContract],
      });
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      STPacket.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return ST Packet hash', () => {
      const serializedPacket = '123';
      const hashedPacket = '456';

      STPacket.prototype.serialize.returns(serializedPacket);

      hashMock.returns(hashedPacket);

      const result = stPacket.hash();

      expect(result).to.equal(hashedPacket);

      expect(STPacket.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedPacket);
    });
  });
});
