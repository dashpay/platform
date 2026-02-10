import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { identifier } from './mocks/Identity/index.js';

before(async () => {
  await initWasm();
});

describe('TokenDistributionRecipient', () => {
  describe('ContractOwner()', () => {
    it('should create ContractOwner recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
    });
  });

  describe('Identity()', () => {
    it('should create Identity recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
    });
  });

  describe('EvonodesByParticipation()', () => {
    it('should create EvonodesByParticipation recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
    });
  });

  describe('recipientType', () => {
    it('should return ContractOwner for ContractOwner recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      expect(recipient.recipientType).to.equal('ContractOwner');
    });

    it('should return Identity type for Identity recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient.recipientType).to.equal(`Identity(${identifier})`);
    });

    it('should return EvonodesByParticipation for EvonodesByParticipation recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient.recipientType).to.equal('EvonodesByParticipation');
    });
  });

  describe('value', () => {
    it('should return undefined for ContractOwner recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      expect(recipient.value).to.equal(undefined);
    });

    it('should return identifier for Identity recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient.value.toBase58()).to.equal(identifier);
    });

    it('should return undefined for EvonodesByParticipation recipient', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient.value).to.equal(undefined);
    });
  });
});
