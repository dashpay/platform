const { expect } = require('chai');
const WalletStore = require('./WalletStore');

let walletStore;
describe('WalletStore - Class', ()=> {
  describe('simple usage', () => {
    it('should create a walletStore', function () {
      walletStore = new WalletStore('squawk7700');
      expect(walletStore.walletId).to.equal('squawk7700');
    });
    it('should create path state', function () {
      walletStore.createPathState('m/0')
      expect(walletStore.state.paths.get('m/0')).to.deep.equal({
        path: 'm/0',
        addresses: {}
      });
      // TODO: Can be done later to have a better way to update path state
      walletStore.state.paths.get('m/0').addresses['m/0'] = 'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7'
    });

    it('should get path state', function () {
      const pathState = walletStore.getPathState('m/0');
      expect(pathState).to.deep.equal({
        path: 'm/0',
        addresses: {
          'm/0': 'yTwEca67QSkZ6axGdpNFzWPaCj8zqYybY7'
        }
      });
    });
    it('should insert identity', function () {
      const identityId = 'abcde1234';
      const identityIndex = 0;
      walletStore.insertIdentityIdAtIndex(identityId, identityIndex);
    });
    it('should get indexed identity ids', function () {
      expect(walletStore.getIndexedIdentityIds()).to.deep.equal(['abcde1234'])
    });
    it('should get identity id by index', function () {
      expect(walletStore.getIdentityIdByIndex(0)).to.deep.equal('abcde1234')
    });
    it('should export and import state', function () {
      const exportedState = walletStore.exportState();
      const importedWalletStore = new WalletStore();
      importedWalletStore.importState(exportedState);
      expect(exportedState).to.deep.equal(importedWalletStore.exportState())
    });
  })
})
