import {HDPublicKey, Mnemonic, Network, Plugins, PrivateKey} from "../types";
import {Account} from '../Account/Account';
import {HDPrivateKey} from "@dashevo/dashcore-lib";

export declare class Wallet {
    offlineMode: boolean;
    allowSensitiveOperations: boolean;
    injectDefaultPlugins: boolean;
    plugins: [Plugins];
    passphrase?: string;

    constructor(options?: Wallet.Options);

    createAccount(accOptions: Account.Options): Promise<Account>;

    disconnect(): void;

    exportWallet(): Mnemonic["toString"];

    fromMnemonic(mnemonic: Mnemonic): void;

    fromPrivateKey(privateKey: PrivateKey): void;

    fromHDPrivateKey(privateKey: HDPrivateKey): void;

    fromHDPublicKey(HDPublicKey: HDPublicKey): void;

    fromSeed(seed: string): void;

    generateNewWalletId(): void;

    getAccount(accOptions?: Wallet.getAccOptions): Promise<Account>;

    updateNetwork(network: Network): boolean;
}

declare interface DAPIClientOptions {
    dapiAddressProvider?: any;
    addresses?: Array<any | string>;
    seeds?: Array<any | string>;
    network?: string;
    networkType?: string;
    timeout?: number;
    retries?: number;
    baseBanTime?: number;
}


export declare namespace Wallet {

    interface Options {
        debug?: boolean;
        offlineMode?: boolean;
        transport?: DAPIClientOptions | Transport;
        network?: Network;
        plugins?: [Plugins];
        passphrase?: string;
        injectDefaultPlugins?: boolean;
        mnemonic?: Mnemonic | string | null;
        seed?: Mnemonic | string;
        privateKey?: PrivateKey | string;
        HDPrivateKey?: HDPrivateKey | string;
        HDPublicKey?: HDPublicKey | string;
    }

    interface getAccOptions extends Account.Options {
        index?: number;
    }
}


