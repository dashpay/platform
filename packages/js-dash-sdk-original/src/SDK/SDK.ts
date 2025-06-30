/* eslint-disable no-restricted-exports */
// TODO remove default export
import { default as _DAPIClient } from '@dashevo/dapi-client';
import {
  Wallet as _Wallet,
  Account as _Account,
  DerivableKeyChain as _KeyChain,
  CONSTANTS as _WalletLibCONSTANTS,
  EVENTS as _WalletLibEVENTS,
  utils as _WalletLibUtils,
  plugins as _WalletLibPlugins,
} from '@dashevo/wallet-lib';
import { Client as _Client } from './Client';
import { Core as _Core } from './Core';
import { Platform as _Platform } from './Platform';

import { StateTransitionBroadcastError } from '../errors/StateTransitionBroadcastError';

export namespace SDK {
  export const DAPIClient = _DAPIClient;
  export const Client = _Client;

  export const Core = _Core;
  export const Platform = _Platform;

  // Wallet-lib primitives
  export const Wallet = _Wallet;
  export const Account = _Account;
  export const KeyChain = _KeyChain;

  // TODO: consider merging into Wallet above and mark as DEPRECATED
  export const WalletLib = {
    CONSTANTS: _WalletLibCONSTANTS,
    EVENTS: _WalletLibEVENTS,
    plugins: _WalletLibPlugins,
    utils: _WalletLibUtils,
  };

  export const PlatformProtocol = Platform.DashPlatformProtocol;

  export const Essentials = {
    Buffer,
  };

  export const Errors = {
    StateTransitionBroadcastError,
  };
}
export { SDK as default };
