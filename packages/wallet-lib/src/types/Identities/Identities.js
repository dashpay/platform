import _ from 'lodash';
import Wallet from '../Wallet/Wallet.js';

class Identities {
  constructor(wallet) {
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');

    this.walletId = wallet.walletId;

    this.storage = wallet.storage;

    this.keyChain = wallet.keyChainStore.getMasterKeyChain();
  }
}

import _Identities_getIdentityHDKeyById from './methods/getIdentityHDKeyById.js';
Identities.prototype.getIdentityHDKeyById = _Identities_getIdentityHDKeyById;
import _Identities_getIdentityHDKeyByIndex from './methods/getIdentityHDKeyByIndex.js';
Identities.prototype.getIdentityHDKeyByIndex = _Identities_getIdentityHDKeyByIndex;
import _Identities_getIdentityIds from './methods/getIdentityIds.js';
Identities.prototype.getIdentityIds = _Identities_getIdentityIds;

export default Identities;