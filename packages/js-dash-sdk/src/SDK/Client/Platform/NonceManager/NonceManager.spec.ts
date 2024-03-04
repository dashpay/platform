import { Identifier } from '@dashevo/wasm-dpp';
import { expect } from 'chai';
import NonceManager, { NONCE_FETCH_INTERVAL } from './NonceManager';

describe('Dash - NonceManager', () => {
  let nonceManager: NonceManager;
  let dapiClientMock;
  const identityId = new Identifier(Buffer.alloc(32).fill(1));
  const contractId = new Identifier(Buffer.alloc(32).fill(2));

  beforeEach(function beforeEach() {
    dapiClientMock = {
      platform: {
        getIdentityContractNonce: this.sinon.stub(),
        getIdentityNonce: this.sinon.stub(),
      },
    };

    nonceManager = new NonceManager(dapiClientMock);
  });

  describe('Identity nonce', () => {
    it('should set and get identity nonce', async () => {
      nonceManager.setIdentityNonce(identityId, 1);
      expect(await nonceManager.getIdentityNonce(identityId)).to.be.equal(1);
      expect(dapiClientMock.platform.getIdentityNonce).to.not.be.called();
    });

    it('should fetch identity nonce if it is not present', async () => {
      dapiClientMock.platform.getIdentityNonce.resolves({ identityNonce: 1 });
      expect(await nonceManager.getIdentityNonce(identityId)).to.be.equal(1);
      expect(dapiClientMock.platform.getIdentityNonce).to.be.calledOnce();
    });

    it('should invalidate and re-fetch nonce after interval passed', async function it() {
      const clock = this.sinon.useFakeTimers();
      dapiClientMock.platform.getIdentityNonce.resolves({ identityNonce: 1 });
      expect(await nonceManager.getIdentityNonce(identityId)).to.be.equal(1);

      clock.tick(NONCE_FETCH_INTERVAL + 1);
      dapiClientMock.platform.getIdentityNonce.resolves({ identityNonce: 2 });
      await nonceManager.getIdentityNonce(identityId);
      expect(await nonceManager.getIdentityNonce(identityId)).to.be.equal(2);
      clock.restore();
    });

    it('should bump identity nonce', async () => {
      dapiClientMock.platform.getIdentityNonce.resolves({ identityNonce: 1 });
      const prevNonce = await nonceManager.getIdentityNonce(identityId);
      const nextNonce = await nonceManager.bumpIdentityNonce(identityId);
      const currentNonce = await nonceManager.getIdentityNonce(identityId);
      expect(nextNonce)
        .to.equal(currentNonce)
        .to.equal(prevNonce + 1);
    });
  });

  describe('Identity contract nonce', () => {
    it('should set and get identity contract nonce', async () => {
      nonceManager.setIdentityContractNonce(identityId, contractId, 1);
      expect(await nonceManager.getIdentityContractNonce(identityId, contractId))
        .to.be.equal(1);
      expect(dapiClientMock.platform.getIdentityContractNonce).to.not.be.called();
    });

    it('should fetch identity contract nonce if it is not present', async () => {
      dapiClientMock.platform.getIdentityContractNonce.resolves({ identityContractNonce: 1 });
      expect(await nonceManager.getIdentityContractNonce(identityId, contractId))
        .to.be.equal(1);
      expect(dapiClientMock.platform.getIdentityContractNonce).to.be.calledOnce();
    });

    it('should invalidate and re-fetch nonce after interval passed', async function it() {
      const clock = this.sinon.useFakeTimers();
      dapiClientMock.platform.getIdentityContractNonce.resolves({ identityContractNonce: 1 });
      expect(await nonceManager.getIdentityContractNonce(identityId, contractId))
        .to.be.equal(1);

      clock.tick(NONCE_FETCH_INTERVAL + 1);
      dapiClientMock.platform.getIdentityContractNonce.resolves({ identityContractNonce: 2 });
      await nonceManager.getIdentityContractNonce(identityId, contractId);
      expect(await nonceManager.getIdentityContractNonce(identityId, contractId))
        .to.be.equal(2);
      clock.restore();
    });

    it('should bump identity contract nonce', async () => {
      dapiClientMock.platform.getIdentityContractNonce.resolves({ identityContractNonce: 1 });
      const prevNonce = await nonceManager.getIdentityContractNonce(identityId, contractId);
      const nextNonce = await nonceManager.bumpIdentityContractNonce(identityId, contractId);
      const currentNonce = await nonceManager.getIdentityContractNonce(identityId, contractId);
      expect(nextNonce)
        .to.equal(currentNonce)
        .to.equal(prevNonce + 1);
    });
  });
});
