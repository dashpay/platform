import { Client as _Client } from './Client';
import { Core as _Core } from './Core';
import { Platform as _Platform } from './Platform';
import { default as _DAPIClient } from '@dashevo/dapi-client';

import { StateTransitionBroadcastError } from '../errors/StateTransitionBroadcastError'

import {
  Wallet as _Wallet,
  Account as _Account,
  DerivableKeyChain as _KeyChain,
  CONSTANTS as _WalletLibCONSTANTS,
  EVENTS as _WalletLibEVENTS,
  utils as _WalletLibUtils,
  plugins as _WalletLibPlugins
} from '@dashevo/wallet-lib';

export namespace SDK {
  export let DAPIClient = _DAPIClient;
  export let Client = _Client;

  export let Core = _Core;
  // TODO: consider marking as DEPRECATED and use PlatformProtocol below instead
  export let Platform = _Platform;

  // Wallet-lib primitives
  export let Wallet = _Wallet;
  export let Account = _Account;
  export let KeyChain = _KeyChain;

  // TODO: consider merging into Wallet above and mark as DEPRECATED
  export let WalletLib = {
    CONSTANTS: _WalletLibCONSTANTS,
    EVENTS: _WalletLibEVENTS,
    plugins: _WalletLibPlugins,
    utils: _WalletLibUtils,
  }

  export let PlatformProtocol = Platform.DashPlatformProtocol;

  export let Essentials = {
    Buffer,
  }

  export let Errors = {
    StateTransitionBroadcastError
  }
}
export { SDK as default };
