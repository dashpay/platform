import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ContractBounds', () => {
  const contractIdHex = '1111111111111111111111111111111111111111111111111111111111111111';
  let contractIdBase58: string;

  before(() => {
    contractIdBase58 = wasm.Identifier.fromHex(contractIdHex).toBase58();
  });

  describe('constructor()', () => {
    it('should create SingleContract bounds without document type', () => {
      const bounds = new wasm.ContractBounds(contractIdBase58);

      expect(bounds.contractBoundsType).to.equal('singleContract');
      expect(bounds.documentTypeName).to.be.undefined();
    });

    it('should create SingleContractDocumentType bounds with document type', () => {
      const bounds = new wasm.ContractBounds(contractIdBase58, 'note');

      expect(bounds.contractBoundsType).to.equal('documentType');
      expect(bounds.documentTypeName).to.equal('note');
    });
  });

  describe('SingleContract()', () => {
    it('should create SingleContract via static method', () => {
      const bounds = wasm.ContractBounds.SingleContract(contractIdBase58);

      expect(bounds.contractBoundsType).to.equal('singleContract');
    });
  });

  describe('SingleContractDocumentType()', () => {
    it('should create SingleContractDocumentType via static method', () => {
      const bounds = wasm.ContractBounds.SingleContractDocumentType(contractIdBase58, 'profile');

      expect(bounds.contractBoundsType).to.equal('documentType');
      expect(bounds.documentTypeName).to.equal('profile');
    });
  });

  describe('toJSON()', () => {
    it('should convert SingleContract to JSON matching fixture', () => {
      const bounds = wasm.ContractBounds.SingleContract(contractIdBase58);

      const json = bounds.toJSON();
      expect(json).to.deep.equal({
        type: 'singleContract',
        id: contractIdBase58,
      });
    });

    it('should convert SingleContractDocumentType to JSON matching fixture', () => {
      const bounds = wasm.ContractBounds.SingleContractDocumentType(contractIdBase58, 'profile');

      const json = bounds.toJSON();
      expect(json).to.deep.equal({
        type: 'documentType',
        id: contractIdBase58,
        documentTypeName: 'profile',
      });
    });
  });

  describe('fromJSON()', () => {
    it('should create SingleContract from JSON fixture and verify getters', () => {
      const fixture = {
        type: 'singleContract',
        id: contractIdBase58,
      };

      const restored = wasm.ContractBounds.fromJSON(fixture);
      expect(restored.contractBoundsType).to.equal('singleContract');
      expect(restored.contractBoundsTypeNumber).to.equal(0);
      expect(restored.identifier.toBase58()).to.equal(contractIdBase58);
      expect(restored.documentTypeName).to.be.undefined();
    });

    it('should create SingleContractDocumentType from JSON fixture and verify getters', () => {
      const fixture = {
        type: 'documentType',
        id: contractIdBase58,
        documentTypeName: 'profile',
      };

      const restored = wasm.ContractBounds.fromJSON(fixture);
      expect(restored.contractBoundsType).to.equal('documentType');
      expect(restored.contractBoundsTypeNumber).to.equal(1);
      expect(restored.identifier.toBase58()).to.equal(contractIdBase58);
      expect(restored.documentTypeName).to.equal('profile');
    });
  });

  describe('toObject()', () => {
    it('should convert SingleContract to Object with Uint8Array identifier', () => {
      const bounds = wasm.ContractBounds.SingleContract(contractIdBase58);

      const obj = bounds.toObject();
      expect(obj.type).to.equal('singleContract');
      expect(obj.id).to.be.instanceOf(Uint8Array);
      expect(wasm.Identifier.fromBytes(obj.id).toHex()).to.equal(contractIdHex);
    });

    it('should convert SingleContractDocumentType to Object with Uint8Array identifier', () => {
      const bounds = wasm.ContractBounds.SingleContractDocumentType(contractIdBase58, 'profile');

      const obj = bounds.toObject();
      expect(obj.type).to.equal('documentType');
      expect(obj.id).to.be.instanceOf(Uint8Array);
      expect(wasm.Identifier.fromBytes(obj.id).toHex()).to.equal(contractIdHex);
      expect(obj.documentTypeName).to.equal('profile');
    });
  });

  describe('fromObject()', () => {
    it('should create SingleContract from Object fixture and verify getters', () => {
      const obj = {
        type: 'singleContract',
        id: contractIdBase58,
      };

      const restored = wasm.ContractBounds.fromObject(obj);
      expect(restored.contractBoundsType).to.equal('singleContract');
      expect(restored.contractBoundsTypeNumber).to.equal(0);
      expect(restored.identifier.toHex()).to.equal(contractIdHex);
      expect(restored.documentTypeName).to.be.undefined();
    });

    it('should create SingleContractDocumentType from Object fixture and verify getters', () => {
      const obj = {
        type: 'documentType',
        id: contractIdBase58,
        documentTypeName: 'profile',
      };

      const restored = wasm.ContractBounds.fromObject(obj);
      expect(restored.contractBoundsType).to.equal('documentType');
      expect(restored.contractBoundsTypeNumber).to.equal(1);
      expect(restored.identifier.toHex()).to.equal(contractIdHex);
      expect(restored.documentTypeName).to.equal('profile');
    });
  });

  describe('identifier', () => {
    it('should return identifier', () => {
      const bounds = new wasm.ContractBounds(contractIdBase58);

      expect(bounds.identifier).to.be.an('object');
      expect(bounds.identifier.__type).to.equal('Identifier');
    });

    it('should throw an error when setting invalid identifier via setter', () => {
      const bounds = new wasm.ContractBounds(contractIdBase58);

      expect(() => {
        bounds.identifier = 'invalid';
      }).to.throw();
    });
  });

  describe('contractBoundsTypeNumber', () => {
    it('should return contractBoundsTypeNumber', () => {
      const singleContract = wasm.ContractBounds.SingleContract(contractIdBase58);
      expect(singleContract.contractBoundsTypeNumber).to.equal(0);

      const documentType = wasm.ContractBounds.SingleContractDocumentType(contractIdBase58, 'note');
      expect(documentType.contractBoundsTypeNumber).to.equal(1);
    });
  });
});
