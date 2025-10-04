import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('ChangeControlRules', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create rules from values', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      expect(noOne.__wbg_ptr).to.not.equal(0);
      expect(changeRules.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get authorizedToMakeChange', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      expect(changeRules.authorizedToMakeChange.constructor.name).to.deep.equal('AuthorizedActionTakersWASM');
    });

    it('should allow to get adminActionTakers', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      expect(changeRules.adminActionTakers.constructor.name).to.deep.equal('AuthorizedActionTakersWASM');
    });

    it('should allow to get changingAuthorizedActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      expect(changeRules.changingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(true);
    });

    it('should allow to get changingAdminActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      expect(changeRules.changingAdminActionTakersToNoOneAllowed).to.deep.equal(true);
    });

    it('should allow to get selfChangingAdminActionTakersAllowed', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      expect(changeRules.selfChangingAdminActionTakersAllowed).to.deep.equal(true);
    });
  });

  describe('setters', () => {
    it('should allow to set authorizedToMakeChange', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const newActionTaker = wasm.AuthorizedActionTakersWASM.ContractOwner();

      changeRules.authorizedToMakeChange = newActionTaker;

      expect(changeRules.authorizedToMakeChange.constructor.name).to.deep.equal('AuthorizedActionTakersWASM');
      expect(changeRules.authorizedToMakeChange.getTakerType()).to.deep.equal('ContractOwner');
      expect(newActionTaker.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to set adminActionTakers', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const newActionTaker = wasm.AuthorizedActionTakersWASM.ContractOwner();

      changeRules.adminActionTakers = newActionTaker;

      expect(changeRules.adminActionTakers.constructor.name).to.deep.equal('AuthorizedActionTakersWASM');
      expect(changeRules.adminActionTakers.getTakerType()).to.deep.equal('ContractOwner');
      expect(newActionTaker.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to set changingAuthorizedActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      changeRules.changingAuthorizedActionTakersToNoOneAllowed = false;

      expect(changeRules.changingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set changingAdminActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      changeRules.changingAdminActionTakersToNoOneAllowed = false;

      expect(changeRules.changingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set selfChangingAdminActionTakersAllowed', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      changeRules.selfChangingAdminActionTakersAllowed = false;

      expect(changeRules.selfChangingAdminActionTakersAllowed).to.deep.equal(false);
    });
  });
});
