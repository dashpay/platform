import { WalletStore } from '../WalletStore/WalletStore'

export declare class Storage {
    constructor(options?: Storage.Options);

    getWalletStore(walletId: string): WalletStore
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
