const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const Metadata = require('@dashevo/dpp/lib/Metadata');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

describe('Document', () => {
  let document;
  let dataContract;
  let metadataFixture;

  beforeEach(() => {
    dataContract = getDataContractFixture();
    [document] = getDocumentsFixture(dataContract).slice(8);

    metadataFixture = new Metadata({
      blockHeight: 42,
      coreChainLockedHeight: 0,
      timeMs: new Date().getTime(),
      protocolVersion: 1,
    });

    document.setMetadata(metadataFixture);
  });

  describe('#toJSON', () => {
    it('should return json document', () => {
      const result = document.toJSON();

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId().toString(),
        $id: document.getId().toString(),
        $ownerId: getDocumentsFixture.ownerId.toString(),
        $revision: 1,
        $type: 'withByteArrays',
        byteArrayField: document.get('byteArrayField').toString('base64'),
        identifierField: document.get('identifierField').toString(),
      });
    });
  });

  describe('#toObject', () => {
    it('should return raw document', () => {
      const result = document.toObject();

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId(),
        $id: document.getId(),
        $ownerId: getDocumentsFixture.ownerId,
        $revision: 1,
        $type: 'withByteArrays',
        byteArrayField: document.get('byteArrayField'),
        identifierField: document.get('identifierField'),
      });
    });

    it('should return raw document with Identifiers', () => {
      const result = document.toObject({ skipIdentifiersConversion: true });

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId(),
        $id: document.getId(),
        $ownerId: getDocumentsFixture.ownerId,
        $revision: 1,
        $type: 'withByteArrays',
        byteArrayField: document.get('byteArrayField'),
        identifierField: document.get('identifierField'),
      });

      expect(result.$dataContractId).to.be.an.instanceOf(Identifier);
      expect(result.$id).to.be.an.instanceOf(Identifier);
      expect(result.$ownerId).to.be.an.instanceOf(Identifier);
      expect(result.identifierField).to.be.an.instanceOf(Identifier);
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata', () => {
      const otherMetadata = new Metadata(43, 1);

      document.setMetadata(otherMetadata);

      expect(document.metadata).to.deep.equal(otherMetadata);
    });
  });

  describe('#getMetadata', () => {
    it('should get metadata', () => {
      expect(document.getMetadata()).to.deep.equal(metadataFixture);
    });
  });
});
