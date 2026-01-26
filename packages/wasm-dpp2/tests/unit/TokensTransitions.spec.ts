import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { dataContractId, ownerId } from './mocks/Document/index.js';

before(async () => {
  await initWasm();
});

let baseTransition: InstanceType<typeof wasm.TokenBaseTransition>;

describe('TokenTransitions', () => {
  before(async () => {
    baseTransition = new wasm.TokenBaseTransition({
      identityContractNonce: BigInt(1),
      tokenContractPosition: 1,
      dataContractId,
      tokenId: ownerId,
    });
  });

  describe('TokenBurnTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

        expect(burnTransition.constructor.name).to.equal('TokenBurnTransition');
        expect(burnTransition).to.be.an.instanceof(wasm.TokenBurnTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('burnAmount', () => {
      it('should return burnAmount', () => {
        const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

        expect(burnTransition.burnAmount).to.equal(BigInt(11));
      });

      it('should set burnAmount', () => {
        const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

        burnTransition.burnAmount = BigInt(222);

        expect(burnTransition.burnAmount).to.equal(BigInt(222));
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

        expect(burnTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

        expect(burnTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const burnTransition = new wasm.TokenBurnTransition(baseTransition, BigInt(11), 'bbbb');

        burnTransition.publicNote = 'aaaa';

        expect(burnTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenMintTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

        expect(mintTransition.constructor.name).to.equal('TokenMintTransition');
        expect(mintTransition).to.be.an.instanceof(wasm.TokenMintTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('amount', () => {
      it('should return amount', () => {
        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

        expect(mintTransition.amount).to.equal(BigInt(11));
      });

      it('should set amount', () => {
        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

        mintTransition.amount = BigInt(222);

        expect(mintTransition.amount).to.equal(BigInt(222));
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

        expect(mintTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

        expect(mintTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(11), 'bbbb');

        mintTransition.publicNote = 'aaaa';

        expect(mintTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenTransferTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
        });

        expect(transferTransition.constructor.name).to.equal('TokenTransferTransition');
        expect(transferTransition).to.be.an.instanceof(wasm.TokenTransferTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });

      it('should create instance with shared encrypted note', () => {
        const sharedEncryptedNote = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);

        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
          sharedEncryptedNote,
        });

        expect(sharedEncryptedNote.constructor.name).to.equal('SharedEncryptedNote');
        expect(transferTransition.constructor.name).to.equal('TokenTransferTransition');
        expect(transferTransition).to.be.an.instanceof(wasm.TokenTransferTransition);
        expect(sharedEncryptedNote).to.be.an.instanceof(wasm.SharedEncryptedNote);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });

      it('should create instance with private encrypted note', () => {
        const privateEncryptedNote = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
          privateEncryptedNote,
        });

        expect(privateEncryptedNote.constructor.name).to.equal('PrivateEncryptedNote');
        expect(transferTransition.constructor.name).to.equal('TokenTransferTransition');
        expect(transferTransition).to.be.an.instanceof(wasm.TokenTransferTransition);
        expect(privateEncryptedNote).to.be.an.instanceof(wasm.PrivateEncryptedNote);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
        });

        expect(transferTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('amount', () => {
      it('should return amount', () => {
        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
        });

        expect(transferTransition.amount).to.equal(BigInt(11));
      });

      it('should set amount', () => {
        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
        });

        transferTransition.amount = BigInt(222);

        expect(transferTransition.amount).to.equal(BigInt(222));
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
        });

        expect(transferTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
        });

        transferTransition.publicNote = 'aaaa';

        expect(transferTransition.publicNote).to.equal('aaaa');
      });
    });

    describe('sharedEncryptedNote', () => {
      it('should return sharedEncryptedNote', () => {
        const sharedEncryptedNote = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);

        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
          sharedEncryptedNote,
        });

        expect(transferTransition.sharedEncryptedNote.constructor.name).to.equal('SharedEncryptedNote');
      });

      it('should set sharedEncryptedNote', () => {
        const sharedEncryptedNote = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);

        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
          sharedEncryptedNote,
        });

        const sharedEncryptedNote2 = new wasm.SharedEncryptedNote(0, 0, [0, 0, 0]);
        transferTransition.sharedEncryptedNote = sharedEncryptedNote2;

        expect(transferTransition.sharedEncryptedNote.constructor.name).to.equal('SharedEncryptedNote');
        expect(sharedEncryptedNote2).to.be.an.instanceof(wasm.SharedEncryptedNote);
      });
    });

    describe('privateEncryptedNote', () => {
      it('should return privateEncryptedNote', () => {
        const privateEncryptedNote = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
          privateEncryptedNote,
        });

        expect(transferTransition.privateEncryptedNote.constructor.name).to.equal('PrivateEncryptedNote');
      });

      it('should set privateEncryptedNote', () => {
        const privateEncryptedNote = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);

        const transferTransition = new wasm.TokenTransferTransition({
          base: baseTransition,
          recipientId: ownerId,
          amount: BigInt(11),
          publicNote: 'bbbb',
          privateEncryptedNote,
        });

        const privateEncryptedNote2 = new wasm.PrivateEncryptedNote(0, 0, [0, 0, 0]);
        transferTransition.privateEncryptedNote = privateEncryptedNote2;

        expect(transferTransition.privateEncryptedNote.constructor.name).to.equal('PrivateEncryptedNote');
        expect(privateEncryptedNote2).to.be.an.instanceof(wasm.PrivateEncryptedNote);
      });
    });
  });

  describe('TokenFreezeTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const freezeTransition = new wasm.TokenFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(freezeTransition.constructor.name).to.equal('TokenFreezeTransition');
        expect(freezeTransition).to.be.an.instanceof(wasm.TokenFreezeTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const freezeTransition = new wasm.TokenFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(freezeTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('frozenIdentityId', () => {
      it('should return frozenIdentityId', () => {
        const freezeTransition = new wasm.TokenFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(freezeTransition.frozenIdentityId.toBase58()).to.equal(ownerId);
      });

      it('should set frozenIdentityId', () => {
        const freezeTransition = new wasm.TokenFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        freezeTransition.frozenIdentityId = dataContractId;

        expect(freezeTransition.frozenIdentityId.toBase58()).to.equal(dataContractId);
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const freezeTransition = new wasm.TokenFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(freezeTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const freezeTransition = new wasm.TokenFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        freezeTransition.publicNote = 'aaaa';

        expect(freezeTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenUnFreezeTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const unfreezeTransition = new wasm.TokenUnFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(unfreezeTransition.constructor.name).to.equal('TokenUnFreezeTransition');
        expect(unfreezeTransition).to.be.an.instanceof(wasm.TokenUnFreezeTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const unfreezeTransition = new wasm.TokenUnFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(unfreezeTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('frozenIdentityId', () => {
      it('should return frozenIdentityId', () => {
        const unfreezeTransition = new wasm.TokenUnFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(unfreezeTransition.frozenIdentityId.toBase58()).to.equal(ownerId);
      });

      it('should set frozenIdentityId', () => {
        const unfreezeTransition = new wasm.TokenUnFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        unfreezeTransition.frozenIdentityId = dataContractId;

        expect(unfreezeTransition.frozenIdentityId.toBase58()).to.equal(dataContractId);
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const unfreezeTransition = new wasm.TokenUnFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(unfreezeTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const unfreezeTransition = new wasm.TokenUnFreezeTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        unfreezeTransition.publicNote = 'aaaa';

        expect(unfreezeTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenDestroyFrozenFundsTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(tokenDestroyFrozenFundsTransition.constructor.name).to.equal('TokenDestroyFrozenFundsTransition');
        expect(tokenDestroyFrozenFundsTransition).to.be.an.instanceof(wasm.TokenDestroyFrozenFundsTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(tokenDestroyFrozenFundsTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('frozenIdentityId', () => {
      it('should return frozenIdentityId', () => {
        const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(tokenDestroyFrozenFundsTransition.frozenIdentityId.toBase58()).to.equal(ownerId);
      });

      it('should set frozenIdentityId', () => {
        const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        tokenDestroyFrozenFundsTransition.frozenIdentityId = dataContractId;

        expect(tokenDestroyFrozenFundsTransition.frozenIdentityId.toBase58()).to.equal(dataContractId);
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        expect(tokenDestroyFrozenFundsTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const tokenDestroyFrozenFundsTransition = new wasm.TokenDestroyFrozenFundsTransition(
          baseTransition,
          ownerId,
          'bbbb',
        );

        tokenDestroyFrozenFundsTransition.publicNote = 'aaaa';

        expect(tokenDestroyFrozenFundsTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenClaimTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
          wasm.TokenDistributionType.PreProgrammed,
          'bbbb',
        );

        expect(claimTransition.constructor.name).to.equal('TokenClaimTransition');
        expect(claimTransition).to.be.an.instanceof(wasm.TokenClaimTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });

      it('should create instance without distribution type', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
        );

        expect(claimTransition.constructor.name).to.equal('TokenClaimTransition');
        expect(claimTransition).to.be.an.instanceof(wasm.TokenClaimTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
          wasm.TokenDistributionType.PreProgrammed,
          'bbbb',
        );

        expect(claimTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('distributionType', () => {
      it('should return distributionType', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
          wasm.TokenDistributionType.PreProgrammed,
          'bbbb',
        );

        expect(claimTransition.distributionType).to.equal('PreProgrammed');
      });

      it('should set distributionType', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
          wasm.TokenDistributionType.Perpetual,
          'bbbb',
        );

        claimTransition.distributionType = wasm.TokenDistributionType.Perpetual;

        expect(claimTransition.distributionType).to.equal('Perpetual');
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
          wasm.TokenDistributionType.PreProgrammed,
          'bbbb',
        );

        expect(claimTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const claimTransition = new wasm.TokenClaimTransition(
          baseTransition,
          wasm.TokenDistributionType.Perpetual,
          'bbbb',
        );

        claimTransition.publicNote = 'aaaa';

        expect(claimTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenEmergencyActionTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
          baseTransition,
          wasm.TokenDistributionType.PreProgrammed,
          'bbbb',
        );

        expect(emergencyActionTransition.constructor.name).to.equal('TokenEmergencyActionTransition');
        expect(emergencyActionTransition).to.be.an.instanceof(wasm.TokenEmergencyActionTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
          baseTransition,
          wasm.TokenEmergencyAction.Pause,
          'bbbb',
        );

        expect(emergencyActionTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('emergencyAction', () => {
      it('should return emergencyAction', () => {
        const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
          baseTransition,
          wasm.TokenEmergencyAction.Pause,
          'bbbb',
        );

        expect(emergencyActionTransition.emergencyAction).to.equal('Pause');
      });

      it('should set emergencyAction', () => {
        const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
          baseTransition,
          wasm.TokenEmergencyAction.Pause,
          'bbbb',
        );

        emergencyActionTransition.emergencyAction = wasm.TokenEmergencyAction.Resume;

        expect(emergencyActionTransition.emergencyAction).to.equal('Resume');
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
          baseTransition,
          wasm.TokenEmergencyAction.Pause,
          'bbbb',
        );

        expect(emergencyActionTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const emergencyActionTransition = new wasm.TokenEmergencyActionTransition(
          baseTransition,
          wasm.TokenEmergencyAction.Pause,
          'bbbb',
        );

        emergencyActionTransition.publicNote = 'aaaa';

        expect(emergencyActionTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenConfigUpdateTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const tradeMode = wasm.TokenTradeMode.NotTradeable();

        const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
          baseTransition,
          wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
          'bbbb',
        );

        expect(configUpdateTransition.constructor.name).to.equal('TokenConfigUpdateTransition');
        expect(configUpdateTransition).to.be.an.instanceof(wasm.TokenConfigUpdateTransition);
        expect(tradeMode).to.be.an.instanceof(wasm.TokenTradeMode);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const tradeMode = wasm.TokenTradeMode.NotTradeable();

        const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
          baseTransition,
          wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
          'bbbb',
        );

        expect(configUpdateTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('updateTokenConfigurationItem', () => {
      it('should return updateTokenConfigurationItem', () => {
        const tradeMode = wasm.TokenTradeMode.NotTradeable();

        const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
          baseTransition,
          wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
          'bbbb',
        );

        expect(configUpdateTransition.updateTokenConfigurationItem.constructor.name).to.equal('TokenConfigurationChangeItem');
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const tradeMode = wasm.TokenTradeMode.NotTradeable();

        const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
          baseTransition,
          wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
          'bbbb',
        );

        expect(configUpdateTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        // At this moment available only one trade mode
        const tradeMode = wasm.TokenTradeMode.NotTradeable();

        const configUpdateTransition = new wasm.TokenConfigUpdateTransition(
          baseTransition,
          wasm.TokenConfigurationChangeItem.MarketplaceTradeModeItem(tradeMode),
          'bbbb',
        );

        configUpdateTransition.publicNote = 'aaaa';

        expect(configUpdateTransition.publicNote).to.equal('aaaa');
      });
    });
  });

  describe('TokenDirectPurchaseTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
          baseTransition,
          BigInt(111),
          BigInt(111),
        );

        expect(directPurchaseTransition.constructor.name).to.equal('TokenDirectPurchaseTransition');
        expect(directPurchaseTransition).to.be.an.instanceof(wasm.TokenDirectPurchaseTransition);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
          baseTransition,
          BigInt(111),
          BigInt(111),
        );

        expect(directPurchaseTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('tokenCount', () => {
      it('should return tokenCount', () => {
        const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
          baseTransition,
          BigInt(111),
          BigInt(111),
        );

        expect(directPurchaseTransition.tokenCount).to.equal(BigInt(111));
      });

      it('should set tokenCount', () => {
        const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
          baseTransition,
          BigInt(111),
          BigInt(111),
        );

        directPurchaseTransition.tokenCount = BigInt(222);

        expect(directPurchaseTransition.tokenCount).to.equal(BigInt(222));
      });
    });

    describe('totalAgreedPrice', () => {
      it('should return totalAgreedPrice', () => {
        const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
          baseTransition,
          BigInt(111),
          BigInt(111),
        );

        expect(directPurchaseTransition.totalAgreedPrice).to.equal(BigInt(111));
      });

      it('should set totalAgreedPrice', () => {
        const directPurchaseTransition = new wasm.TokenDirectPurchaseTransition(
          baseTransition,
          BigInt(111),
          BigInt(111),
        );

        directPurchaseTransition.totalAgreedPrice = BigInt(222);

        expect(directPurchaseTransition.totalAgreedPrice).to.equal(BigInt(222));
      });
    });
  });

  describe('TokenSetPriceForDirectPurchaseTransition', () => {
    describe('constructor', () => {
      it('should create instance from values', () => {
        const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

        const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
          baseTransition,
          price,
          'bbbb',
        );

        expect(price.constructor.name).to.equal('TokenPricingSchedule');
        expect(setPriceDirectPurchaseTransition.constructor.name).to.equal('TokenSetPriceForDirectPurchaseTransition');
        expect(setPriceDirectPurchaseTransition).to.be.an.instanceof(wasm.TokenSetPriceForDirectPurchaseTransition);
        expect(price).to.be.an.instanceof(wasm.TokenPricingSchedule);
        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

        const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
          baseTransition,
          price,
          'bbbb',
        );

        expect(setPriceDirectPurchaseTransition.base.constructor.name).to.equal('TokenBaseTransition');
      });
    });

    describe('price', () => {
      it('should return price', () => {
        const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

        const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
          baseTransition,
          price,
          'bbbb',
        );

        expect(setPriceDirectPurchaseTransition.price.constructor.name).to.equal('TokenPricingSchedule');
      });

      it('should set price', () => {
        const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

        const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
          baseTransition,
          price,
          'bbbb',
        );

        setPriceDirectPurchaseTransition.price = wasm.TokenPricingSchedule.SetPrices({ 101: 1010 });

        expect(setPriceDirectPurchaseTransition.price.constructor.name).to.equal('TokenPricingSchedule');
      });
    });

    describe('publicNote', () => {
      it('should return publicNote', () => {
        const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

        const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
          baseTransition,
          price,
          'bbbb',
        );

        expect(setPriceDirectPurchaseTransition.publicNote).to.equal('bbbb');
      });

      it('should set publicNote', () => {
        const price = wasm.TokenPricingSchedule.SetPrices({ 100: 1000 });

        const setPriceDirectPurchaseTransition = new wasm.TokenSetPriceForDirectPurchaseTransition(
          baseTransition,
          price,
          'bbbb',
        );

        setPriceDirectPurchaseTransition.publicNote = 'aaaa';

        expect(setPriceDirectPurchaseTransition.publicNote).to.equal('aaaa');
      });
    });
  });
});
