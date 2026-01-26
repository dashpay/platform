import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

interface ChangeControlRulesOptions {
  isChangingAuthorizedActionTakersToNoOneAllowed?: boolean;
  isChangingAdminActionTakersToNoOneAllowed?: boolean;
  isSelfChangingAdminActionTakersAllowed?: boolean;
}

describe('ChangeControlRules', () => {
  // Helper function to create ChangeControlRules with default options
  function createChangeControlRules(
    authorizedToMakeChange: InstanceType<typeof wasm.AuthorizedActionTakers>,
    adminActionTakers: InstanceType<typeof wasm.AuthorizedActionTakers>,
    options: ChangeControlRulesOptions = {},
  ) {
    return new wasm.ChangeControlRules({
      authorizedToMakeChange,
      adminActionTakers,
      isChangingAuthorizedActionTakersToNoOneAllowed: options.isChangingAuthorizedActionTakersToNoOneAllowed ?? true,
      isChangingAdminActionTakersToNoOneAllowed: options.isChangingAdminActionTakersToNoOneAllowed ?? true,
      isSelfChangingAdminActionTakersAllowed: options.isSelfChangingAdminActionTakersAllowed ?? true,
    });
  }

  describe('serialization / deserialization', () => {
    it('should allow to create rules from values', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(noOne).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(changeRules).to.be.an.instanceof(wasm.ChangeControlRules);
    });
  });

  describe('getters', () => {
    it('should allow to get authorizedToMakeChange', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.authorizedToMakeChange.constructor.name).to.deep.equal('AuthorizedActionTakers');
    });

    it('should allow to get adminActionTakers', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.adminActionTakers.constructor.name).to.deep.equal('AuthorizedActionTakers');
    });

    it('should allow to get changingAuthorizedActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(true);
    });

    it('should allow to get changingAdminActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(true);
    });

    it('should allow to get selfChangingAdminActionTakersAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(true);
    });
  });

  describe('setters', () => {
    it('should allow to set authorizedToMakeChange', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const newActionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      changeRules.authorizedToMakeChange = newActionTaker;

      expect(changeRules.authorizedToMakeChange.constructor.name).to.deep.equal('AuthorizedActionTakers');
      expect(changeRules.authorizedToMakeChange.takerType).to.deep.equal('ContractOwner');
      expect(newActionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
    });

    it('should allow to set adminActionTakers', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const newActionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      changeRules.adminActionTakers = newActionTaker;

      expect(changeRules.adminActionTakers.constructor.name).to.deep.equal('AuthorizedActionTakers');
      expect(changeRules.adminActionTakers.takerType).to.deep.equal('ContractOwner');
      expect(newActionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
    });

    it('should allow to set changingAuthorizedActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      changeRules.isChangingAuthorizedActionTakersToNoOneAllowed = false;

      expect(changeRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set changingAdminActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      changeRules.isChangingAdminActionTakersToNoOneAllowed = false;

      expect(changeRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set selfChangingAdminActionTakersAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      changeRules.isSelfChangingAdminActionTakersAllowed = false;

      expect(changeRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(false);
    });
  });
});
