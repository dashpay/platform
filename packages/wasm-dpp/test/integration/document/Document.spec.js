const MetadataJs = require('@dashevo/dpp/lib/Metadata');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const { default: loadWasmDpp } = require('../../../dist');

let Document;
let DataContract;
let Metadata;
let Identifier;

describe('Document', () => {
  let document;
  let dataContract;
  let metadataFixture;

  beforeEach(async () => {
    ({
      Document,
      DataContract,
      Metadata,
      Identifier,
    } = await loadWasmDpp());

    const dataContractJs = getDataContractFixture();
    dataContract = new DataContract(dataContractJs.toObject());

    const [documentJs] = getDocumentsFixture(dataContractJs).slice(8);
    document = new Document(documentJs.toObject(), dataContract);

    const metadataFixtureJs = new MetadataJs({
      blockHeight: 42,
      coreChainLockedHeight: 0,
      timeMs: new Date().getTime(),
      protocolVersion: 1,
    });

    metadataFixture = Metadata.from({
      blockHeight: 42,
      coreChainLockedHeight: 0,
      timeMs: new Date().getTime(),
      protocolVersion: 1,
    });

    documentJs.setMetadata(metadataFixtureJs);
    document.setMetadata(metadataFixture);
  });

  describe('#toJSON', () => {
    it('should return json document - Rust', () => {
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
    it('should return raw document - Rust', () => {
      const result = document.toObject();

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId().toBuffer(),
        $id: document.getId().toBuffer(),
        $ownerId: getDocumentsFixture.ownerId,
        $revision: 1,
        $type: 'withByteArrays',
        byteArrayField: document.get('byteArrayField'),
        identifierField: document.get('identifierField').toBuffer(),
      });
    });

    it('should return raw document with Identifiers - Rust', () => {
      const result = document.toObject({ skipIdentifiersConversion: true });

      expect(result.$dataContractId).to.be.an.instanceOf(Identifier);
      expect(result.$id).to.be.an.instanceOf(Identifier);
      expect(result.$ownerId).to.be.an.instanceOf(Identifier);
      expect(result.identifierField).to.be.an.instanceOf(Identifier);

      expect(result.$dataContractId.toBuffer()).to.deep.equal(dataContract.getId().toBuffer());
      expect(result.$id.toBuffer()).to.deep.equal(document.getId().toBuffer());
      expect(result.$ownerId.toBuffer()).to.deep.equal(getDocumentsFixture.ownerId.toBuffer());
      expect(result.identifierField.toBuffer()).to.deep.equal(document.get('identifierField').toBuffer());
      expect(result.$protocolVersion).to.deep.equal(document.getProtocolVersion());
      expect(result.$revision).to.deep.equal(document.getRevision());
      expect(result.$type).to.deep.equal(document.getType());
      expect(result.byteArrayField).to.deep.equal(document.get('byteArrayField'));
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata - Rust', () => {
      const otherMetadata = new Metadata({
        blockHeight: 43,
        coreChainLockedHeight: 1,
        timeMs: 100,
        protocolVersion: 2,
      });
      document.setMetadata(otherMetadata);

      expect(document.getMetadata().toObject()).to.deep.equal(otherMetadata.toObject());
    });
  });

  describe('#getMetadata', () => {
    it('should get metadata - Rust', () => {
      expect(document.getMetadata().toObject()).to.deep.equal(metadataFixture.toObject());
    });
  });
});
