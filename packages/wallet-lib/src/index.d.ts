/// <reference path="types/types.d.ts" />
/// <reference path="types/Account/Account.d.ts" />
/// <reference path="types/Wallet/Wallet.d.ts" />
import { Account } from "./types/Account/Account";
import { Wallet } from "./types/Wallet/Wallet";
import { Identities } from "./types/Identities/Identities";
import { ChainStore } from "./types/ChainStore/ChainStore";
import { DerivableKeyChain } from "./types/DerivableKeyChain/DerivableKeyChain";
import { KeyChainStore } from "./types/KeyChainStore/KeyChainStore";

// @ts-ignore
import * as CONSTANTS from "./CONSTANTS";
// @ts-ignore
import * as EVENTS from "./EVENTS";
import * as utils from "./utils";
import * as plugins from "./plugins";

export {
  Account,
  Wallet,
  ChainStore,
  DerivableKeyChain,
  KeyChainStore,
  Identities,
  EVENTS,
  CONSTANTS,
  utils,
  plugins,
};
declare module '@dashevo/wallet-lib';
