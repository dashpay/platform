/// <reference path="types/types.d.ts" />
/// <reference path="types/Account/Account.d.ts" />
/// <reference path="types/Wallet/Wallet.d.ts" />
import { Account } from "./types/Account/Account";
import { Wallet } from "./types/Wallet/Wallet";
import { KeyChain } from "./types/KeyChain/KeyChain";
import CONSTANTS from "./CONSTANTS";
import EVENTS from "./EVENTS";
import utils from "./utils";
import plugins from "./plugins";

export {
    Account,
    Wallet,
    KeyChain,
    EVENTS,
    CONSTANTS,
    utils,
    plugins,
};
declare module '@dashevo/wallet-lib';
