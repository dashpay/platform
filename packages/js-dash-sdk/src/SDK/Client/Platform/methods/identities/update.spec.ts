import { expect } from 'chai';
import NonceManager from '../../NonceManager/NonceManager';
import { update } from './update';

describe('Platform', () => {
  describe('Identities', () => {
    describe('#update', () => {
      it('should leave the identity nonce unchanged when the update is refused', async function it() {
        const identityId: any = { toString: () => 'identity' };
        const nonceManager = new NonceManager({
          platform: {
            getIdentityNonce: this.sinon.stub().resolves({ identityNonce: BigInt(5) }),
          },
        } as any);
        const refusal = new Error('cannot disable authentication key 2');
        const platformMock: any = {
          logger: { debug: () => {}, silly: () => {} },
          initialize: async () => {},
          nonceManager,
          dpp: {
            identity: {
              createIdentityUpdateTransition: this.sinon.stub().throws(refusal),
            },
          },
        };
        const identity: any = { getId: () => identityId };
        const keyToDisable: any = { getId: () => 2 };
        const privateKeys: any = {};

        await expect(
          update.call(platformMock, identity, { disable: [keyToDisable] }, privateKeys),
        ).to.be.rejectedWith(refusal);

        expect(await nonceManager.getIdentityNonce(identityId)).to.equal(BigInt(5));
      });
    });
  });
});
