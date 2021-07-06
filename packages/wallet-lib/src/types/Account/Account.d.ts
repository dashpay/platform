import {
    Transaction,
    AddressObj,
    AddressInfo,
    AddressType,
    transactionId,
    PublicAddress,
    PrivateKey,
    Strategy,
    Network,
    Plugins, RawTransaction, TransactionsMap, WalletObj, StatusInfo
} from "../types";
import { KeyChain } from "../KeyChain/KeyChain";
import { InstantLock } from "@dashevo/dashcore-lib";
import { Wallet } from "../../index";
import { Transport } from "../../transport/Transport";
import { BlockHeader } from "@dashevo/dashcore-lib/typings/block/BlockHeader";
import { UnspentOutput } from "@dashevo/dashcore-lib/typings/transaction/UnspentOutput";
import { Storage } from "../Storage/Storage";

export declare class Account {
    constructor(wallet: Wallet, options?: Account.Options);

    index: number;
    injectDefaultPlugins?: boolean;
    allowSensitiveOperations?: boolean;
    debug?: boolean;
    cacheTx?: boolean;
    cacheBlockHeaders?: boolean;
    label?: string | null;
    strategy?: Strategy;
    keyChain: KeyChain;
    state: any;
    storage: Storage;
    store: Storage.store;
    walletId: string;
    transport: Transport;

    isReady(): Promise<boolean>;
    isInitialized(): Promise<boolean>;
    getBIP44Path(network?: Network, index?: number): string;
    getNetwork(): Network;

    broadcastTransaction(rawtx: Transaction|RawTransaction): Promise<transactionId>;
    connect(): boolean;
    createTransaction(opts: Account.createTransactionOptions): Transaction;
    decode(method: string, data: any): any;
    decrypt(method: string, data: any, secret: string, encoding?: "hex"|string): string;
    encrypt(method: string, data: any, secret: string): string;
    disconnect(): Promise<Boolean>;
    fetchAddressInfo(addressObj: AddressObj, fetchUtxo: boolean): Promise<AddressInfo | false>;
    fetchStatus(): Promise<StatusInfo>;
    forceRefreshAccount(): boolean;
    generateAddress(path: string): AddressObj;
    getAddress(index: number, _type: AddressType): AddressObj;
    getAddresses(_type: AddressType): [AddressObj];
    getBlockHeader(identifier: string|number):Promise<BlockHeader>
    getConfirmedBalance(displayDuffs?: boolean): number;
    getPlugin(name: string): Object;
    getPrivateKeys(addressList: [PublicAddress]): [PrivateKey];
    getTotalBalance(displayDuffs?: boolean): number;
    getTransaction(txid: transactionId): Transaction;
    getTransactions(): [Transaction];
    getUTXOS(): [UnspentOutput];
    getUnconfirmedBalance(displayDuffs?: boolean): number;
    getUnusedAddress(type?: AddressType, skip?: number): AddressObj;
    getUnusedIdentityIndex(): Promise<number>;
    getWorker(workerName: string): Object;
    hasPlugins([Plugin]): {found:Boolean, results:[{name: string}]};
    injectPlugin(unsafePlugin: Plugins, allowSensitiveOperation?: boolean, awaitOnInjection?: boolean): Promise<any>;
    sign(object: Transaction, privateKeys: [PrivateKey], sigType?: number): Transaction;
    waitForInstantLock(string: transactionHash): Promise<InstantLock>;
}

export declare interface RecipientOptions {
    satoshis?: number;
    amount?: number;
    address: PublicAddress,
}

export declare namespace Account {
    interface Options {
        index?: number,
        network?: Network;
        debug?: boolean;
        label?: string;
        plugins?: [Plugins];
        cacheBlockHeaders?: boolean;
        cacheTx?: boolean;
        allowSensitiveOperations?: boolean;
        injectDefaultPlugins?: boolean;
        strategy?: Strategy;
        cache?:{
            transactions?:TransactionsMap,
            addresses?:WalletObj["addresses"]
        }
    }

    interface createTransactionOptions {
        recipient?: PublicAddress,
        satoshis?: number,
        amount?: number,

        recipients?: [RecipientOptions]

        change?: string;
        utxos?: [object];
        isInstantSend?: boolean;
        deductFee?: boolean
        privateKeys?: [PrivateKey],
        strategy?: Strategy
    }

}
