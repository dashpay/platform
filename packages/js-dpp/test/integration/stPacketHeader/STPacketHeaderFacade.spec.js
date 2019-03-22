const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const STPacketHeader = require('../../../lib/stPacketHeader/STPacketHeader');

const ValidationResult = require('../../../lib/validation/ValidationResult');

describe('STPacketHeaderFacade', () => {
  let dpp;
  let stPacketHeader;

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    stPacketHeader = new STPacketHeader(
      '4b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b75',
      '5b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b23',
      '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b12',
    );
  });

  describe('create', () => {
    it('should create ST Packet Header', () => {
      const result = dpp.packetHeader.create(
        stPacketHeader.getContractId(),
        stPacketHeader.getItemsMerkleRoot(),
        stPacketHeader.getItemsHash(),
      );

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.getContractId()).to.equal(stPacketHeader.getContractId());
      expect(result.getItemsMerkleRoot()).to.equal(stPacketHeader.getItemsMerkleRoot());
      expect(result.getItemsHash()).to.equal(stPacketHeader.getItemsHash());
    });
  });

  describe('createFromObject', () => {
    it('should create ST Packet Header from plain object', () => {
      const result = dpp.packetHeader.createFromObject(stPacketHeader.toJSON());

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.toJSON()).to.deep.equal(stPacketHeader.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should create ST Packet Header from string', () => {
      const result = dpp.packetHeader.createFromSerialized(stPacketHeader.serialize());

      expect(result).to.be.an.instanceOf(STPacketHeader);

      expect(result.toJSON()).to.deep.equal(stPacketHeader.toJSON());
    });
  });

  describe('validate', () => {
    it('should validate ST Packet Header', () => {
      const result = dpp.packetHeader.validate(stPacketHeader.toJSON());

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
