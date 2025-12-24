import getWasm from './helpers/wasm.js';
import { dataContractId, ownerId } from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

let baseTransition;

describe('TokenTransitions', () => {
  before(async () => {
    baseTransition = new wasm.TokenBaseTransition(BigInt(1), 1, dataContractId, ownerId);
  });

  describe('serialize/deserialize', () => {
    it('should allow to create burn transition', () => {
      const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

      expect(burnTransition.constructor.name).to.equal('TokenBurnTransition');
      expect(burnTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create mint transition', () => {
      const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

      expect(mintTransition.constructor.name).to.equal('TokenMintTransition');
      expect(mintTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create transfer transition', () => {
      const transferTransition = new wasm.TokenTransferTransition(
        baseTransition,
        ownerId,
        BigInt(11),
        'bbbb',
      );

      expect(transferTransition.constructor.name).to.equal('TokenTransferTransition');
      expect(transferTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create transfer transition with shared encrypted note', () => {
      const sharedEncryptedNote = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);

      const transferTransition = new wasm.TokenTransferTransition(
        baseTransition,
        ownerId,
        BigInt(11),
        'bbbb',
        sharedEncryptedNote,
      );

      expect(sharedEncryptedNote.constructor.name).to.equal('SharedEncryptedNote');
      expect(transferTransition.constructor.name).to.equal('TokenTransferTransition');
      expect(transferTransition.__wbg_ptr).to.not.equal(0);
      expect(sharedEncryptedNote.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create transfer transition with private encrypted note', () => {
      const privateEncryptedNote = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

      const transferTransition = new wasm.TokenTransferTransition(
        baseTransition,
        ownerId,
        BigInt(11),
        'bbbb',
        undefined,
        privateEncryptedNote,
      );

      expect(privateEncryptedNote.constructor.name).to.equal('PrivateEncryptedNote');
      expect(transferTransition.constructor.name).to.equal('TokenTransferTransition');
      expect(transferTransition.__wbg_ptr).to.not.equal(0);
      expect(privateEncryptedNote.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create freeze transition', () => {
      const freezeTransition = new wasm.TokenFreezeTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      expect(freezeTransition.constructor.name).to.equal('TokenFreezeTransition');
      expect(freezeTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create unfreeze transition', () => {
      const unfreezeTransition = new wasm.TokenUnFreezeTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      expect(unfreezeTransition.constructor.name).to.equal('TokenUnFreezeTransition');
      expect(unfreezeTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create destroy frozen funds transition', () => {
      const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      expect(tokenDestroyFrozenFundsTransition.constructor.name).to.equal('TokenDestroyFrozenFundsTransition');
      expect(tokenDestroyFrozenFundsTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create claim transition', () => {
      const claimTransition = new wasm.TokenClaimTransition(
        baseTransition,
        wasm.TokenDistributionType.PreProgrammed,
        'bbbb',
      );

      expect(claimTransition.constructor.name).to.equal('TokenClaimTransition');
      expect(claimTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create claim transition without distribution type', () => {
      const claimTransition = new wasm.TokenClaimTransition(
        baseTransition,
      );

      expect(claimTransition.constructor.name).to.equal('TokenClaimTransition');
      expect(claimTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create emergency action transition', () => {
      const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
        baseTransition,
        wasm.TokenDistributionType.PreProgrammed,
        'bbbb',
      );

      expect(emergencyActionTransition.constructor.name).to.equal('TokenEmergencyActionTransition');
      expect(emergencyActionTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create config update transition', () => {
      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
        baseTransition,
        wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
        'bbbb',
      );

      expect(configUpdateTransition.constructor.name).to.equal('TokenConfigUpdateTransition');
      expect(configUpdateTransition.__wbg_ptr).to.not.equal(0);
      expect(tradeMode.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create direct purchase transition', () => {
      const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
        baseTransition,
        BigInt(111),
        BigInt(111),
      );

      expect(directPurchaseTransition.constructor.name).to.equal('TokenDirectPurchaseTransition');
      expect(directPurchaseTransition.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create set price direct purchase transition', () => {
      const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

      const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
        baseTransition,
        price,
        'bbbb',
      );

      expect(price.constructor.name).to.equal('TokenPricingSchedule');
      expect(setPriceDirectPurchaseTransition.constructor.name).to.equal('TokenSetPriceForDirectPurchaseTransition');
      expect(setPriceDirectPurchaseTransition.__wbg_ptr).to.not.equal(0);
      expect(price.__wbg_ptr).to.not.equal(0);
      expect(baseTransition.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to read getters burn transition', () => {
      const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

      expect(burnTransition.burnAmount).to.equal(BigInt(11));
      expect(burnTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(burnTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters mint transition', () => {
      const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

      expect(mintTransition.amount).to.equal(BigInt(11));
      expect(mintTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(mintTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters transfer transition', () => {
      const sharedEncryptedNote = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);
      const privateEncryptedNote = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

      const transferTransition = new wasm.TokenTransferTransition(
        baseTransition,
        ownerId,
        BigInt(11),
        'bbbb',
        sharedEncryptedNote,
        privateEncryptedNote,
      );

      expect(transferTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(transferTransition.amount).to.equal(BigInt(11));
      expect(transferTransition.publicNote).to.equal('bbbb');
      expect(transferTransition.sharedEncryptedNote.constructor.name).to.equal('SharedEncryptedNote');
      expect(transferTransition.privateEncryptedNote.constructor.name).to.equal('PrivateEncryptedNote');
    });

    it('should allow to read getters freeze transition', () => {
      const freezeTransition = new wasm.TokenFreezeTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      expect(freezeTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(freezeTransition.frozenIdentityId.base58()).to.equal(ownerId);
      expect(freezeTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters unfreeze transition', () => {
      const unfreezeTransition = new wasm.TokenUnFreezeTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      expect(unfreezeTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(unfreezeTransition.frozenIdentityId.base58()).to.equal(ownerId);
      expect(unfreezeTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters destroy frozen funds transition', () => {
      const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      expect(tokenDestroyFrozenFundsTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(tokenDestroyFrozenFundsTransition.frozenIdentityId.base58()).to.equal(ownerId);
      expect(tokenDestroyFrozenFundsTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters claim transition', () => {
      const claimTransition = new wasm.TokenClaimTransition(
        baseTransition,
        wasm.TokenDistributionType.PreProgrammed,
        'bbbb',
      );

      expect(claimTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(claimTransition.distributionType).to.equal('PreProgrammed');
      expect(claimTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters emergency action transition', () => {
      const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
        baseTransition,
        wasm.TokenEmergencyAction.Pause,
        'bbbb',
      );

      expect(emergencyActionTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(emergencyActionTransition.emergencyAction).to.equal('Pause');
      expect(emergencyActionTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters config update transition', () => {
      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
        baseTransition,
        wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
        'bbbb',
      );

      expect(configUpdateTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(configUpdateTransition.updateTokenConfigurationItem.constructor.name).to.equal('TokenConfigurationChangeItem');
      expect(configUpdateTransition.publicNote).to.equal('bbbb');
    });

    it('should allow to read getters direct purchase transition', () => {
      const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
        baseTransition,
        BigInt(111),
        BigInt(111),
      );

      expect(directPurchaseTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(directPurchaseTransition.tokenCount).to.equal(BigInt(111));
      expect(directPurchaseTransition.totalAgreedPrice).to.equal(BigInt(111));
    });

    it('should allow to read getters set price direct purchase transition', () => {
      const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

      const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
        baseTransition,
        price,
        'bbbb',
      );

      expect(setPriceDirectPurchaseTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(setPriceDirectPurchaseTransition.price.constructor.name).to.equal('TokenPricingSchedule');
      expect(setPriceDirectPurchaseTransition.publicNote).to.equal('bbbb');
    });
  });

  describe('setters', () => {
    it('should allow to set values burn transition', () => {
      const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

      burnTransition.burnAmount = BigInt(222);
      burnTransition.publicNote = 'aaaa';

      expect(burnTransition.burnAmount).to.equal(BigInt(222));
      expect(burnTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(burnTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values mint transition', () => {
      const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

      mintTransition.amount = BigInt(222);
      mintTransition.publicNote = 'aaaa';

      expect(mintTransition.amount).to.equal(BigInt(222));
      expect(mintTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(mintTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values transfer transition', () => {
      const sharedEncryptedNote = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);
      const privateEncryptedNote = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

      const transferTransition = new wasm.TokenTransferTransition(
        baseTransition,
        ownerId,
        BigInt(11),
        'bbbb',
        sharedEncryptedNote,
        privateEncryptedNote,
      );

      const sharedEncryptedNote2 = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);
      const privateEncryptedNote2 = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

      transferTransition.sharedEncryptedNote = sharedEncryptedNote2;
      transferTransition.privateEncryptedNote = privateEncryptedNote2;
      transferTransition.amount = BigInt(222);
      transferTransition.publicNote = 'aaaa';

      expect(transferTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(transferTransition.amount).to.equal(BigInt(222));
      expect(transferTransition.publicNote).to.equal('aaaa');
      expect(transferTransition.sharedEncryptedNote.constructor.name).to.equal('SharedEncryptedNote');
      expect(transferTransition.privateEncryptedNote.constructor.name).to.equal('PrivateEncryptedNote');
      expect(sharedEncryptedNote2.__wbg_ptr).to.not.equal(0);
      expect(privateEncryptedNote2.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to set values freeze transition', () => {
      const freezeTransition = new wasm.TokenFreezeTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      freezeTransition.frozenIdentityId = dataContractId;
      freezeTransition.publicNote = 'aaaa';

      expect(freezeTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(freezeTransition.frozenIdentityId.base58()).to.equal(dataContractId);
      expect(freezeTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values unfreeze transition', () => {
      const unfreezeTransition = new wasm.TokenUnFreezeTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      unfreezeTransition.frozenIdentityId = dataContractId;
      unfreezeTransition.publicNote = 'aaaa';

      expect(unfreezeTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(unfreezeTransition.frozenIdentityId.base58()).to.equal(dataContractId);
      expect(unfreezeTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values destroy frozen funds transition', () => {
      const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
        baseTransition,
        ownerId,
        'bbbb',
      );

      tokenDestroyFrozenFundsTransition.frozenIdentityId = dataContractId;
      tokenDestroyFrozenFundsTransition.publicNote = 'aaaa';

      expect(tokenDestroyFrozenFundsTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(tokenDestroyFrozenFundsTransition.frozenIdentityId.base58()).to.equal(dataContractId);
      expect(tokenDestroyFrozenFundsTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values claim transition', () => {
      const claimTransition = new wasm.TokenClaimTransition(
        baseTransition,
        wasm.TokenDistributionType.Perpetual,
        'bbbb',
      );

      claimTransition.distributionType = wasm.TokenDistributionType.Perpetual;
      claimTransition.publicNote = 'aaaa';

      expect(claimTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(claimTransition.distributionType).to.equal('Perpetual');
      expect(claimTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values emergency action transition', () => {
      const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
        baseTransition,
        wasm.TokenEmergencyAction.Pause,
        'bbbb',
      );

      emergencyActionTransition.emergencyAction = wasm.TokenEmergencyAction.Resume;
      emergencyActionTransition.publicNote = 'aaaa';

      expect(emergencyActionTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(emergencyActionTransition.emergencyAction).to.equal('Resume');
      expect(emergencyActionTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values config update transition', () => {
      // At this moment available only one trade mode
      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
        baseTransition,
        wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
        'bbbb',
      );

      configUpdateTransition.publicNote = 'aaaa';

      expect(configUpdateTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(configUpdateTransition.updateTokenConfigurationItem.constructor.name).to.equal('TokenConfigurationChangeItem');
      expect(configUpdateTransition.publicNote).to.equal('aaaa');
    });

    it('should allow to set values direct purchase transition', () => {
      const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
        baseTransition,
        BigInt(111),
        BigInt(111),
      );

      directPurchaseTransition.tokenCount = BigInt(222);
      directPurchaseTransition.totalAgreedPrice = BigInt(222);

      expect(directPurchaseTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(directPurchaseTransition.tokenCount).to.equal(BigInt(222));
      expect(directPurchaseTransition.totalAgreedPrice).to.equal(BigInt(222));
    });

    it('should allow to set values set price direct purchase transition', () => {
      const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

      const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
        baseTransition,
        price,
        'bbbb',
      );

      setPriceDirectPurchaseTransition.price = wasm.TokenPricingSchedule.SetPrices({ 101: 1010 });
      setPriceDirectPurchaseTransition.publicNote = 'aaaa';

      expect(setPriceDirectPurchaseTransition.base.constructor.name).to.equal('TokenBaseTransition');
      expect(setPriceDirectPurchaseTransition.price.constructor.name).to.equal('TokenPricingSchedule');
      expect(setPriceDirectPurchaseTransition.publicNote).to.equal('aaaa');
    });
  });
});
