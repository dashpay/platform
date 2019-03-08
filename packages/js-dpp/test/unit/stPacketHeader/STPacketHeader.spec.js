const rewiremock = require('rewiremock/node');

describe('STPacketHeader', () => {
  let hashMock;
  let encodeMock;
  let STPacketHeader;
  let stPacketHeader;
  let dpContractId;
  let itemsMerkleRoot;
  let itemsHash;
  let rawSTPacketHeader;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    // Require STPacketHeader module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/stPacketHeader/STPacketHeader');

    STPacketHeader = rewiremock.proxy('../../../lib/stPacketHeader/STPacketHeader', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    dpContractId = '6b74011f5d2ad1a8d45b71b9702f542054535653593c3cfbba3fdadeca278288';
    itemsMerkleRoot = '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b';
    itemsHash = 'y90b273ff34fce19d6b804eff5a3f5747ada4eaa22f86fj5jf652ddb78755642';

    stPacketHeader = new STPacketHeader(
      dpContractId,
      itemsMerkleRoot,
      itemsHash,
    );

    rawSTPacketHeader = {
      contractId: dpContractId,
      itemsMerkleRoot,
      itemsHash,
    };
  });

  describe('constructor', () => {
    it('should return new ST Packet Header with specified Contract ID', () => {
      expect(stPacketHeader).to.be.an.instanceOf(STPacketHeader);

      expect(stPacketHeader.contractId).to.equal(dpContractId);
      expect(stPacketHeader.itemsMerkleRoot).to.equal(itemsMerkleRoot);
      expect(stPacketHeader.itemsHash).to.equal(itemsHash);
    });
  });

  describe('#setDPContractId', () => {
    it('should set DP Contract ID', () => {
      const result = stPacketHeader.setDPContractId(dpContractId);

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.contractId).to.equal(dpContractId);
    });
  });

  describe('#getDPContractId', () => {
    it('should return DP Contract ID', () => {
      const result = stPacketHeader.getDPContractId();

      expect(result).to.equal(dpContractId);
    });
  });

  describe('#setItemsMerkleRoot', () => {
    it('should set items merkle root', () => {
      const result = stPacketHeader.setItemsMerkleRoot(itemsMerkleRoot);

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.itemsMerkleRoot).to.equal(itemsMerkleRoot);
    });
  });

  describe('#getItemsMerkleRoot', () => {
    it('should return items merkle root', () => {
      stPacketHeader.itemsMerkleRoot = itemsMerkleRoot;

      const result = stPacketHeader.getItemsMerkleRoot();

      expect(result).to.equal(itemsMerkleRoot);
    });
  });

  describe('#setItemsHash', () => {
    it('should set items hash', () => {
      const result = stPacketHeader.setItemsHash(itemsHash);

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.itemsHash).to.equal(itemsHash);
    });
  });

  describe('#getItemsHash', () => {
    it('should return items hash', () => {
      stPacketHeader.itemsHash = itemsHash;

      const result = stPacketHeader.getItemsHash();

      expect(result).to.equal(itemsHash);
    });
  });

  describe('#toJSON', () => {
    it('should return ST Packet Header as plain object', () => {
      const result = stPacketHeader.toJSON();

      expect(result).to.deep.equal(rawSTPacketHeader);
    });
  });

  describe('#serialize', () => {
    it('should return serialized ST Packet Header', () => {
      const serializedSTPacket = '123';

      encodeMock.returns(serializedSTPacket);

      const result = stPacketHeader.serialize();

      expect(result).to.equal(serializedSTPacket);

      expect(encodeMock).to.have.been.calledOnceWith(rawSTPacketHeader);
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      STPacketHeader.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return ST Packet Header hash', () => {
      const serializedPacket = '123';
      const hashedPacket = '456';

      STPacketHeader.prototype.serialize.returns(serializedPacket);

      hashMock.returns(hashedPacket);

      const result = stPacketHeader.hash();

      expect(result).to.equal(hashedPacket);

      expect(STPacketHeader.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedPacket);
    });
  });
});
