import getWasm from './helpers/wasm.js';
import { identifier } from './mocks/Identity/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenDistributionRecipient', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values ContractOwner', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
    });

    it('should allow to create from values Identity', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
    });

    it('should allow to create from values EvonodesByParticipation', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
    });
  });

  describe('getters', () => {
    it('should allow to get values ContractOwner', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      expect(recipient.recipientType).to.equal('ContractOwner');
      expect(recipient.value).to.equal(undefined);
    });

    it('should allow to get values Identity', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient.recipientType).to.equal(`Identity(${identifier})`);
      expect(recipient.value.toBase58()).to.equal(identifier);
    });

    it('should allow to get values EvonodesByParticipation', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient.recipientType).to.equal('EvonodesByParticipation');
      expect(recipient.value).to.equal(undefined);
    });
  });
});
