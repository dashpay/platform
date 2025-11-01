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

      expect(recipient.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create from values Identity', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create from values EvonodesByParticipation', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get values ContractOwner', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      expect(recipient.getType()).to.equal('ContractOwner');
      expect(recipient.getValue()).to.equal(undefined);
    });

    it('should allow to get values Identity', () => {
      const recipient = wasm.TokenDistributionRecipient.Identity(identifier);

      expect(recipient.getType()).to.equal(`Identity(${identifier})`);
      expect(recipient.getValue().base58()).to.equal(identifier);
    });

    it('should allow to get values EvonodesByParticipation', () => {
      const recipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      expect(recipient.getType()).to.equal('EvonodesByParticipation');
      expect(recipient.getValue()).to.equal(undefined);
    });
  });
});
