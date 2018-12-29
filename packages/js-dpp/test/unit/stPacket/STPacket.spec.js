const rewiremock = require('rewiremock/node');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');
const getDapObjectsFixture = require('../../../lib/test/fixtures/getDapObjectsFixture');

const DapContract = require('../../../lib/dapContract/DapContract');
const DapObject = require('../../../lib/dapObject/DapObject');

const ContractAndObjectsNotAllowedSamePacketError = require('../../../lib/stPacket/errors/ContractAndObjectsNotAllowedSamePacketError');

describe('STPacket', () => {
  let hashMock;
  let encodeMock;
  let STPacket;
  let dapContract;
  let dapObjects;
  let stPacket;
  let itemsHash;
  let itemsMerkleRoot;
  let dapContractId;
  let merkleTreeUtilMock;
  let getMerkleTreeMock;
  let getMerkleRootMock;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;
    getMerkleTreeMock = this.sinonSandbox.stub();
    getMerkleRootMock = this.sinonSandbox.stub();
    merkleTreeUtilMock = {
      getMerkleTree: getMerkleTreeMock,
      getMerkleRoot: getMerkleRootMock,
    };

    STPacket = rewiremock.proxy('../../../lib/stPacket/STPacket', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
      '../../../lib/util/merkleTree': merkleTreeUtilMock,
      '../../../lib/dapContract/DapContract': DapContract,
      '../../../lib/dapObject/DapObject': DapObject,
    });

    dapContract = getDapContractFixture();
    dapObjects = getDapObjectsFixture();

    dapContractId = dapContract.getId();
    itemsHash = '14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293';
    itemsMerkleRoot = '44207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc292';

    stPacket = new STPacket(dapContractId);
  });

  describe('constructor', () => {
    it('should return new ST Packet with specified DAP Contract ID', () => {
      expect(stPacket).to.be.instanceOf(STPacket);

      expect(stPacket.contractId).to.be.equal(dapContractId);
    });

    it('should return new STPacket with specified DAP Contract ID and DAP Contract', () => {
      const result = new STPacket(dapContractId, dapContract);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.getDapContractId()).to.be.equal(dapContractId);
      expect(result.getDapContract()).to.be.equal(dapContract);
    });

    it('should return new STPacket with specified DAP Contract ID and DAP Objects', () => {
      const result = new STPacket(dapContractId, dapObjects);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.getDapContractId()).to.be.equal(dapContractId);
      expect(result.getDapObjects()).to.be.equal(dapObjects);
    });
  });

  describe('#setDapContractId', () => {
    it('should set Dap Contract ID', () => {
      const contractId = dapContractId;

      const result = stPacket.setDapContractId(contractId);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.contractId).to.be.equal(contractId);
    });
  });

  describe('#getDapContractId', () => {
    it('should return Dap Contract ID', () => {
      const result = stPacket.getDapContractId();

      expect(result).to.be.equal(dapContractId);
    });
  });

  describe('#getItemsMerkleRoot', () => {
    it('should return items merkle root', () => {
      stPacket.setDapContract(dapContract);
      getMerkleRootMock.returns(Buffer.from(itemsMerkleRoot, 'hex'));
      getMerkleTreeMock.returns([Buffer.from(itemsHash, 'hex')]);
      const result = stPacket.getItemsMerkleRoot();

      expect(result).to.be.equal(itemsMerkleRoot);
    });
    it('should return null hash if no items found', () => {
      const result = stPacket.getItemsMerkleRoot();

      expect(result).to.be.equal(null);
    });
  });

  describe('#getItemsHash', () => {
    it('should return items hash', () => {
      hashMock.returns(itemsHash);
      stPacket.setDapContract(dapContract);
      const result = stPacket.getItemsHash();

      expect(result).to.be.equal(itemsHash);
    });
    it('should return null hash if no items found', () => {
      const result = stPacket.getItemsMerkleRoot();

      expect(result).to.be.equal(null);
    });
  });

  describe('#setDapContract', () => {
    it('should set Dap Contract', () => {
      const result = stPacket.setDapContract(dapContract);

      expect(result).to.be.instanceOf(STPacket);

      expect(stPacket.contracts).to.have.lengthOf(1);
      expect(stPacket.contracts[0]).to.be.equal(dapContract);
    });

    it('should throw error if Dap Objects are present', () => {
      stPacket.setDapObjects(dapObjects);

      let error;
      try {
        stPacket.setDapContract(dapContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(ContractAndObjectsNotAllowedSamePacketError);

      expect(error.getSTPacket()).to.be.equal(stPacket);
    });
  });

  describe('#getDapContract', () => {
    it('should return Dap Contract', () => {
      stPacket.contracts = [dapContract];

      const result = stPacket.getDapContract();

      expect(result).to.be.equal(dapContract);
    });

    it('should return null of DAP Contract is not present', () => {
      const result = stPacket.getDapContract();

      expect(result).to.be.null();
    });
  });

  describe('#setDapObjects', () => {
    it('should set DAP Objects and replace previous', () => {
      stPacket.setDapObjects([dapObjects[0]]);

      const result = stPacket.setDapObjects(dapObjects);

      expect(result).to.be.instanceOf(STPacket);

      expect(stPacket.objects).to.be.equal(dapObjects);
    });

    it('should throw error if DAP Contract is present', () => {
      stPacket.setDapContract(dapContract);

      let error;
      try {
        stPacket.setDapObjects(dapObjects);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(ContractAndObjectsNotAllowedSamePacketError);

      expect(error.getSTPacket()).to.be.equal(stPacket);
    });
  });

  describe('#getDapObjects', () => {
    it('should return DAP Objects', () => {
      stPacket.objects = dapObjects;

      const result = stPacket.getDapObjects();

      expect(result).to.be.equal(dapObjects);
    });
  });

  describe('#addDapObject', () => {
    it('should add DAP Object', () => {
      stPacket.addDapObject(dapObjects[0]);

      const result = stPacket.addDapObject(dapObjects[1], dapObjects[2]);

      expect(result).to.be.instanceOf(STPacket);

      expect(stPacket.objects).to.be.deep.equal(dapObjects);
    });
  });

  describe('#toJSON', () => {
    it('should return ST Packet as plain object', () => {
      hashMock.returns(itemsHash);
      getMerkleRootMock.returns(Buffer.from(itemsMerkleRoot, 'hex'));
      getMerkleTreeMock.returns([Buffer.from(itemsHash, 'hex')]);
      stPacket.setDapContract(dapContract);

      const result = stPacket.toJSON();

      expect(result).to.be.deep.equal({
        contractId: dapContractId,
        itemsMerkleRoot,
        itemsHash,
        objects: [],
        contracts: [dapContract.toJSON()],
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized ST Packet', () => {
      hashMock.returns(itemsHash);
      getMerkleRootMock.returns(Buffer.from(itemsMerkleRoot, 'hex'));
      getMerkleTreeMock.returns([Buffer.from(itemsHash, 'hex')]);
      stPacket.setDapContract(dapContract);

      const serializedSTPacket = '123';

      encodeMock.returns(serializedSTPacket);

      const result = stPacket.serialize();

      const rawDapContract = dapContract.toJSON();

      expect(result).to.be.equal(serializedSTPacket);

      expect(encodeMock).to.be.calledTwice();
      expect(encodeMock).to.be.calledWith({
        contractId: dapContractId,
        itemsMerkleRoot,
        itemsHash,
        objects: [],
        contracts: [rawDapContract],
      });
      expect(encodeMock).to.be.calledWith(stPacket.getItemsHashes());
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

      expect(result).to.be.equal(hashedPacket);

      expect(STPacket.prototype.serialize).to.be.calledOnce();

      expect(hashMock).to.be.calledOnceWith(serializedPacket);
    });
  });
});
