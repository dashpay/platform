import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('GroupStateTransitionInfoStatus', () => {
  const actionIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('proposer()', () => {
    it('should create proposer status', () => {
      const status = wasm.GroupStateTransitionInfoStatus.proposer(5);

      expect(status.isProposer).to.be.true();
      expect(status.groupContractPosition).to.equal(5);
      expect(status.actionId).to.be.undefined();
    });
  });

  describe('otherSigner()', () => {
    it('should create other signer status', () => {
      const actionId = wasm.Identifier.fromHex(actionIdHex);
      const status = wasm.GroupStateTransitionInfoStatus.otherSigner(3, actionId);

      expect(status.isProposer).to.be.false();
      expect(status.groupContractPosition).to.equal(3);
      expect(status.actionId).to.not.be.undefined();
      expect(status.actionId.toHex()).to.equal(actionIdHex);
    });
  });

  describe('toInfo()', () => {
    it('should convert proposer to GroupStateTransitionInfo', () => {
      const status = wasm.GroupStateTransitionInfoStatus.proposer(7);
      const info = status.toInfo();

      expect(info).to.be.instanceOf(wasm.GroupStateTransitionInfo);
      expect(info.groupContractPosition).to.equal(7);
      expect(info.isActionProposer).to.be.true();
    });

    it('should convert otherSigner to GroupStateTransitionInfo', () => {
      const actionId = wasm.Identifier.fromHex(actionIdHex);
      const status = wasm.GroupStateTransitionInfoStatus.otherSigner(3, actionId);
      const info = status.toInfo();

      expect(info).to.be.instanceOf(wasm.GroupStateTransitionInfo);
      expect(info.groupContractPosition).to.equal(3);
      expect(info.isActionProposer).to.be.false();
      expect(info.actionId.toHex()).to.equal(actionIdHex);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const status = wasm.GroupStateTransitionInfoStatus.proposer(1);
      expect(status.__type).to.equal('GroupStateTransitionInfoStatus');
    });
  });
});
