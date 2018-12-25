const rewiremock = require('rewiremock/node');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');
const getDapObjectsFixture = require('../../../lib/test/fixtures/getDapObjectsFixture');

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

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    STPacket = rewiremock.proxy('../../../lib/stPacket/STPacket', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    dapContract = getDapContractFixture();
    dapObjects = getDapObjectsFixture();

    dapContractId = dapContract.getId();
    itemsHash = '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b';
    itemsMerkleRoot = 'y90b273ff34fce19d6b804eff5a3f5747ada4eaa22f86fj5jf652ddb78755642';

    stPacket = new STPacket(dapContractId);
    stPacket.setItemsMerkleRoot(itemsMerkleRoot)
      .setItemsHash(itemsHash);
  });

  describe('constructor', () => {
    it('should return new ST Packet with specified Contract ID', () => {
      expect(stPacket).to.be.instanceOf(STPacket);

      expect(stPacket.contractId).to.be.equal(dapContractId);
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

  describe('#setItemsMerkleRoot', () => {
    it('should set items merkle root', () => {
      const result = stPacket.setItemsMerkleRoot(itemsMerkleRoot);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.itemsMerkleRoot).to.be.equal(itemsMerkleRoot);
    });
  });

  describe('#getItemsMerkleRoot', () => {
    it('should return items merkle root', () => {
      const result = stPacket.getItemsMerkleRoot();

      expect(result).to.be.equal(itemsMerkleRoot);
    });
  });

  describe('#setItemsHash', () => {
    it('should set items hash', () => {
      const result = stPacket.setItemsHash(itemsHash);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.itemsHash).to.be.equal(itemsHash);
    });
  });

  describe('#getItemsHash', () => {
    it('should return items hash', () => {
      const result = stPacket.getItemsHash();

      expect(result).to.be.equal(itemsHash);
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
      stPacket.setDapContract(dapContract);

      const serializedSTPacket = '123';

      encodeMock.returns(serializedSTPacket);

      const result = stPacket.serialize();

      expect(result).to.be.equal(serializedSTPacket);

      expect(encodeMock).to.be.calledOnceWith({
        contractId: dapContractId,
        itemsMerkleRoot,
        itemsHash,
        objects: [],
        contracts: [dapContract.toJSON()],
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

      expect(result).to.be.equal(hashedPacket);

      expect(STPacket.prototype.serialize).to.be.calledOnce();

      expect(hashMock).to.be.calledOnceWith(serializedPacket);
    });
  });
});
