const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const { default: loadWasmDpp } = require('../../../dist');

let Metadata;
let Identifier;

describe('ExtendedDocument', () => {
  let document;
  let dataContract;
  let metadataFixture;

  beforeEach(async () => {
    ({
      Metadata,
      Identifier,
    } = await loadWasmDpp());

    dataContract = await getDataContractFixture();

    const documents = await getDocumentsFixture(dataContract);
    [document] = documents.slice(8);

    metadataFixture = Metadata.from({
      blockHeight: 42,
      coreChainLockedHeight: 0,
      timeMs: new Date().getTime(),
      protocolVersion: 1,
    });

    document.setMetadata(metadataFixture);
  });

  describe('#setMetadata', () => {
    it('should set metadata - Rust', () => {
      const otherMetadata = new Metadata(BigInt(43), 1, BigInt(100), 2);
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
