import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('ChangeControlRules', () => {
  // Helper function to create ChangeControlRules with default options
  function createChangeControlRules(authorizedToMakeChange, adminActionTakers, options = {}) {
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

      expect(noOne.__wbg_ptr).to.not.equal(0);
      expect(changeRules.__wbg_ptr).to.not.equal(0);
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
      expect(newActionTaker.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to set adminActionTakers', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const newActionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      changeRules.adminActionTakers = newActionTaker;

      expect(changeRules.adminActionTakers.constructor.name).to.deep.equal('AuthorizedActionTakers');
      expect(changeRules.adminActionTakers.takerType).to.deep.equal('ContractOwner');
      expect(newActionTaker.__wbg_ptr).to.not.equal(0);
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
