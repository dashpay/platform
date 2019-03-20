const rewiremock = require('rewiremock/node');

const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const DPContract = require('../../../lib/contract/DPContract');
const Document = require('../../../lib/document/Document');

const ContractAndDocumentsNotAllowedSamePacketError = require('../../../lib/stPacket/errors/ContractAndDocumentsNotAllowedSamePacketError');

describe('STPacket', () => {
  let hashMock;
  let encodeMock;
  let STPacket;
  let dpContract;
  let documents;
  let stPacket;
  let itemsHash;
  let itemsMerkleRoot;
  let dpContractId;
  let calculateItemsMerkleRootMock;
  let calculateItemsHashMock;

  beforeEach(function beforeEach() {
    dpContract = getDPContractFixture();
    documents = getDocumentsFixture();

    dpContractId = dpContract.getId();
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
      '../../../lib/contract/DPContract': DPContract,
      '../../../lib/document/Document': Document,
      '../../../lib/stPacket/calculateItemsMerkleRoot': calculateItemsMerkleRootMock,
      '../../../lib/stPacket/calculateItemsHash': calculateItemsHashMock,
    });

    stPacket = new STPacket(dpContractId);
  });

  describe('constructor', () => {
    it('should return new ST Packet with specified DP Contract ID', () => {
      expect(stPacket).to.be.an.instanceOf(STPacket);

      expect(stPacket.contractId).to.equal(dpContractId);
    });

    it('should return new STPacket with specified DP Contract ID and DP Contract', () => {
      const result = new STPacket(dpContractId, dpContract);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.getDPContractId()).to.equal(dpContractId);
      expect(result.getDPContract()).to.equal(dpContract);
    });

    it('should return new STPacket with specified DP Contract ID and Documents', () => {
      const result = new STPacket(dpContractId, documents);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.getDPContractId()).to.equal(dpContractId);
      expect(result.getDocuments()).to.equal(documents);
    });
  });

  describe('#setDPContractId', () => {
    it('should set DP Contract ID', () => {
      const contractId = dpContractId;

      const result = stPacket.setDPContractId(contractId);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.contractId).to.equal(contractId);
    });
  });

  describe('#getDPContractId', () => {
    it('should return DP Contract ID', () => {
      const result = stPacket.getDPContractId();

      expect(result).to.equal(dpContractId);
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

  describe('#setDPContract', () => {
    it('should set DP Contract', () => {
      const result = stPacket.setDPContract(dpContract);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(stPacket.contracts).to.have.lengthOf(1);
      expect(stPacket.contracts[0]).to.equal(dpContract);
    });

    it('should throw an error if Documents are present', () => {
      stPacket.setDocuments(documents);

      let error;
      try {
        stPacket.setDPContract(dpContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(ContractAndDocumentsNotAllowedSamePacketError);

      expect(error.getSTPacket()).to.equal(stPacket);
    });
  });

  describe('#getDPContract', () => {
    it('should return DP Contract', () => {
      stPacket.contracts = [dpContract];

      const result = stPacket.getDPContract();

      expect(result).to.equal(dpContract);
    });

    it('should return null of DP Contract is not present', () => {
      const result = stPacket.getDPContract();

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

    it('should throw an error if DP Contract is present', () => {
      stPacket.setDPContract(dpContract);

      let error;
      try {
        stPacket.setDocuments(documents);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(ContractAndDocumentsNotAllowedSamePacketError);

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

      stPacket.setDPContract(dpContract);

      const result = stPacket.toJSON();

      expect(result).to.deep.equal({
        contractId: dpContractId,
        itemsMerkleRoot,
        itemsHash,
        documents: [],
        contracts: [dpContract.toJSON()],
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized ST Packet', () => {
      stPacket.setDPContract(dpContract);

      const serializedSTPacket = '123';

      encodeMock.returns(serializedSTPacket);

      const result = stPacket.serialize();

      const rawDPContract = dpContract.toJSON();

      expect(result).to.equal(serializedSTPacket);

      expect(encodeMock).to.have.been.calledOnceWith({
        contractId: dpContractId,
        itemsMerkleRoot,
        itemsHash,
        documents: [],
        contracts: [rawDPContract],
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
