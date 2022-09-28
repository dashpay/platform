import { Mnemonic, PrivateKey, PublicKey, PublicAddress, Address, HDPublicKey, Network, Plugins } from "../types";
import { Account } from "../Account/Account";
import { Storage } from "../Storage/Storage";
import { HDPrivateKey } from "@dashevo/dashcore-lib";
import { Transport } from "../../transport/Transport";

export declare class Wallet {
    offlineMode: boolean;
    allowSensitiveOperations: boolean;
    injectDefaultPlugins: boolean;
    plugins: [Plugins];
    passphrase?: string;
    transport: Transport;
    network: Network;
    walletId: string;
    accounts: [undefined];
    storage: Storage;
    store: Storage.store;

    constructor(opts:Wallet.IWalletOptions)

    createAccount(accOptions: Account.Options): Promise<Account>;
    disconnect(): void;
    exportWallet():Mnemonic["toString"];
    fromHDPrivateKey(privateKey: HDPrivateKey):void;
    fromHDPublicKey(HDPublicKey:HDPublicKey):void;
    fromMnemonic(mnemonic: Mnemonic):void;
    fromPrivateKey(privateKey: PrivateKey):void;
    fromSeed(seed:string):void;
    generateNewWalletId():string;
    getAccount(accOptions?: Account.Options): Promise<Account>;
    sweepWallet(): Promise<Account>

    /**
     * <b>Warning:</b> Storage dump may contain sensitive data.
     * Please, do not share the output of this function for mainnet wallets.
     * @param options
     */
    dumpStorage(options?: {
        log: boolean
    }): string;
}

declare interface DAPIClientOptions {
    dapiAddressProvider?: any;
    dapiAddresses?: Array<any | string>;
    seeds?: Array<any | string>;
    network?: string;
    networkType?: string;
    timeout?: number;
    retries?: number;
    baseBanTime?: number;
}


export declare namespace Wallet {
    interface IWalletOptions {
        offlineMode?: boolean;
        debug?: boolean;
        transport?: DAPIClientOptions | Transport;
        network?: Network | string;
        plugins?: undefined[]|[Plugins];
        passphrase?: string|null;
        injectDefaultPlugins?: boolean;
        allowSensitiveOperations?: boolean;
        mnemonic?: Mnemonic | string | null;
        seed?: Mnemonic | string;
        privateKey?: PrivateKey | string;
        HDPrivateKey?: HDPrivateKey | string;
        HDPublicKey?: HDPublicKey | string;
        publicKey?: PublicKey | string;
        address?: Address | PublicAddress | string;
        unsafeOptions?: IWalletUnsafeOptions;
        waitForInstantLockTimeout?: number;
        waitForTxMetadataTimeout?: number;
    }

    interface IWalletUnsafeOptions {
        skipSynchronizationBeforeHeight?: number;
    }
}
