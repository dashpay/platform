import { WalletStore } from '../WalletStore/WalletStore'
import {ChainStore} from "../ChainStore/ChainStore";

export declare class Storage {
    constructor(options?: Storage.Options);

    getWalletStore(walletId: string): WalletStore
    getChainStore(network: string): ChainStore
    getDefaultChainStore(): ChainStore
}

export declare namespace Storage {
    interface Options {
        rehydrate: boolean,
        autosave: boolean,
        purgeOnError: boolean,
        autosaveIntervalTime: number,
        network: string
    }
}
