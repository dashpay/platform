import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ContractBounds', () => {
  const contractIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('constructor', () => {
    it('should create SingleContract bounds without document type', () => {
      const bounds = new wasm.ContractBounds(
        Buffer.from(contractIdHex, 'hex'),
      );

      // contractBoundsType returns the serde string representation from rs-dpp
      expect(bounds.contractBoundsType).to.equal('singleContract');
      expect(bounds.documentTypeName).to.be.undefined();
    });

    it('should create SingleContractDocumentType bounds with document type', () => {
      const bounds = new wasm.ContractBounds(
        Buffer.from(contractIdHex, 'hex'),
        'note',
      );

      // contractBoundsType returns the serde string representation from rs-dpp
      expect(bounds.contractBoundsType).to.equal('documentType');
      expect(bounds.documentTypeName).to.equal('note');
    });
  });

  describe('static constructors', () => {
    it('should create SingleContract via static method', () => {
      const bounds = wasm.ContractBounds.SingleContract(
        Buffer.from(contractIdHex, 'hex'),
      );

      expect(bounds.contractBoundsType).to.equal('singleContract');
    });

    it('should create SingleContractDocumentType via static method', () => {
      const bounds = wasm.ContractBounds.SingleContractDocumentType(
        Buffer.from(contractIdHex, 'hex'),
        'profile',
      );

      expect(bounds.contractBoundsType).to.equal('documentType');
      expect(bounds.documentTypeName).to.equal('profile');
    });
  });

  describe('conversion methods', () => {
    it('should round-trip SingleContract via toJSON/fromJSON', () => {
      const bounds = wasm.ContractBounds.SingleContract(
        Buffer.from(contractIdHex, 'hex'),
      );

      const json = bounds.toJSON();
      expect(json).to.be.an('object');
      expect(json.type).to.equal('singleContract');

      const restored = wasm.ContractBounds.fromJSON(json);
      expect(restored.contractBoundsType).to.equal(bounds.contractBoundsType);
      expect(restored.identifier.toBase58()).to.equal(bounds.identifier.toBase58());
    });

    it('should round-trip SingleContractDocumentType via toJSON/fromJSON', () => {
      const bounds = wasm.ContractBounds.SingleContractDocumentType(
        Buffer.from(contractIdHex, 'hex'),
        'profile',
      );

      const json = bounds.toJSON();
      expect(json).to.be.an('object');
      expect(json.type).to.equal('documentType');

      const restored = wasm.ContractBounds.fromJSON(json);
      expect(restored.contractBoundsType).to.equal(bounds.contractBoundsType);
      expect(restored.documentTypeName).to.equal(bounds.documentTypeName);
      expect(restored.identifier.toBase58()).to.equal(bounds.identifier.toBase58());
    });
  });

  describe('getters', () => {
    it('should return identifier', () => {
      const bounds = new wasm.ContractBounds(
        Buffer.from(contractIdHex, 'hex'),
      );

      expect(bounds.identifier).to.be.an('object');
      expect(bounds.identifier.__type).to.equal('Identifier');
    });

    it('should return contractBoundsTypeNumber', () => {
      const singleContract = wasm.ContractBounds.SingleContract(
        Buffer.from(contractIdHex, 'hex'),
      );
      expect(singleContract.contractBoundsTypeNumber).to.equal(0);

      const documentType = wasm.ContractBounds.SingleContractDocumentType(
        Buffer.from(contractIdHex, 'hex'),
        'note',
      );
      expect(documentType.contractBoundsTypeNumber).to.equal(1);
    });
  });

  describe('setters', () => {
    it('should throw an error when setting invalid identifier via setter', () => {
      const bounds = new wasm.ContractBounds(
        Buffer.from(contractIdHex, 'hex'),
      );

      // This setter returns WasmDppResult<()> in Rust
      // Let's test what happens when we pass an invalid identifier (wrong length)
      const invalidIdentifier = Buffer.from('invalid', 'utf8'); // Only 7 bytes, need 32

      expect(() => {
        bounds.identifier = invalidIdentifier;
      }).to.throw();
    });
  });
});
