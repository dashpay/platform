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

  describe('constructor()', () => {
    it('should create rules from values', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(noOne).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(changeRules).to.be.an.instanceof(wasm.ChangeControlRules);
    });
  });

  describe('authorizedToMakeChange', () => {
    it('should return authorizedToMakeChange with correct value', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.authorizedToMakeChange).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(changeRules.authorizedToMakeChange.takerType).to.equal('NoOne');
    });

    it('should set authorizedToMakeChange', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const newActionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      changeRules.authorizedToMakeChange = newActionTaker;

      expect(changeRules.authorizedToMakeChange).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(changeRules.authorizedToMakeChange.takerType).to.equal('ContractOwner');
    });
  });

  describe('adminActionTakers', () => {
    it('should return adminActionTakers with correct value', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.adminActionTakers).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(changeRules.adminActionTakers.takerType).to.equal('NoOne');
    });

    it('should set adminActionTakers', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const newActionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      changeRules.adminActionTakers = newActionTaker;

      expect(changeRules.adminActionTakers).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(changeRules.adminActionTakers.takerType).to.equal('ContractOwner');
    });
  });

  describe('isChangingAuthorizedActionTakersToNoOneAllowed', () => {
    it('should return isChangingAuthorizedActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(true);
    });

    it('should set isChangingAuthorizedActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      changeRules.isChangingAuthorizedActionTakersToNoOneAllowed = false;

      expect(changeRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });
  });

  describe('isChangingAdminActionTakersToNoOneAllowed', () => {
    it('should return isChangingAdminActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(true);
    });

    it('should set isChangingAdminActionTakersToNoOneAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      changeRules.isChangingAdminActionTakersToNoOneAllowed = false;

      expect(changeRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
    });
  });

  describe('isSelfChangingAdminActionTakersAllowed', () => {
    it('should return isSelfChangingAdminActionTakersAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      expect(changeRules.isSelfChangingAdminActionTakersAllowed).to.equal(true);
    });

    it('should set isSelfChangingAdminActionTakersAllowed', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      changeRules.isSelfChangingAdminActionTakersAllowed = false;

      expect(changeRules.isSelfChangingAdminActionTakersAllowed).to.equal(false);
    });
  });
});
