/// <reference types="node" />
import { Wallet as _Wallet, Account as _Account, DerivableKeyChain as _KeyChain } from '@dashevo/wallet-lib';
import { Client as _Client } from './Client';
import { Platform as _Platform } from './Platform';
import { StateTransitionBroadcastError } from '../errors/StateTransitionBroadcastError';
export declare namespace SDK {
    const DAPIClient: any;
    const Client: typeof _Client;
    const Core: any;
    const Platform: typeof _Platform;
    const Wallet: typeof _Wallet;
    const Account: typeof _Account;
    const KeyChain: typeof _KeyChain;
    const WalletLib: {
        CONSTANTS: {
            BIP45: string;
            BIP44: string;
            DUFFS_PER_DASH: number;
            BIP44_ADDRESS_GAP: number;
            SECURE_TRANSACTION_CONFIRMATIONS_NB: number;
            BIP32__ROOT_PATH: string;
            BIP44_LIVENET_ROOT_PATH: string;
            BIP44_TESTNET_ROOT_PATH: string;
            DIP9_LIVENET_ROOT_PATH: string;
            DIP9_TESTNET_ROOT_PATH: string;
            UTXO_SELECTION_MAX_SINGLE_UTXO_FACTOR: number;
            UTXO_SELECTION_MIN_TX_AMOUNT_VS_UTXO_FACTOR: number;
            UTXO_SELECTION_MAX_FEE_VS_TX_AMOUNT_FACTOR: number;
            UTXO_SELECTION_MAX_FEE_VS_SINGLE_UTXO_FEE_FACTOR: number;
            MAX_STANDARD_TX_SIZE: number;
            MAX_P2SH_SIGOPS: number;
            COINBASE_MATURITY: number;
            UTXO_CHAINED_SPENDING_LIMIT_FOR_TX: number;
            FEES: {
                DUST_RELAY_TX_FEE: number;
                ZERO: number;
                ECONOMIC: number;
                NORMAL: number;
                PRIORITY: number;
                INSTANT_FEE_PER_INPUTS: number;
            };
            UNCONFIRMED_TRANSACTION_STATUS_CODE: number;
            WALLET_TYPES: {
                ADDRESS: string;
                PUBLICKEY: string;
                PRIVATEKEY: string;
                SINGLE_ADDRESS: string;
                HDWALLET: string;
                HDPRIVATE: string;
                HDPUBLIC: string;
            };
            INJECTION_LISTS: {
                SAFE_FUNCTIONS: string[];
                UNSAFE_FUNCTIONS: string[];
                UNSAFE_PROPERTIES: string[];
                SAFE_PROPERTIES: string[];
            };
            TRANSACTION_HISTORY_TYPES: {
                RECEIVED: string;
                SENT: string;
                ADDRESS_TRANSFER: string;
                ACCOUNT_TRANSFER: string;
                UNKNOWN: string;
            };
            STORAGE: {
                version: number;
                autosaveIntervalTime: number;
                REORG_SAFE_BLOCKS_COUNT: number;
            };
            TXIN_OUTPOINT_TXID_BYTES: number;
            TXIN_OUTPOINT_INDEX_BYTES: number;
            TXIN_SEQUENCE_BYTES: number;
            TXOUT_DUFFS_VALUE_BYTES: number;
            VERSION_BYTES: number;
            N_LOCKTIME_BYTES: number;
            BLOOM_FALSE_POSITIVE_RATE: number;
            NULL_HASH: string;
        };
        EVENTS: {
            PREFETCHED: string;
            CREATED: string;
            STARTED: string;
            READY: string;
            CONFIRMED_BALANCE_CHANGED: string;
            UNCONFIRMED_BALANCE_CHANGED: string;
            BLOCKHEIGHT_CHANGED: string;
            BLOCK: string;
            TRANSACTION: string;
            BLOCKHEADER: string;
            FETCHED_ADDRESS: string;
            ERROR_UPDATE_ADDRESS: string;
            FETCHED_TRANSACTION: string;
            FETCHED_UNCONFIRMED_TRANSACTION: string;
            FETCHED_CONFIRMED_TRANSACTION: string;
            CONFIRMED_TRANSACTION: string;
            GENERATED_ADDRESS: string;
            DISCOVERY_STARTED: string;
            CONFIGURED: string;
            INITIALIZED: string;
            SAVE_STATE_FAILED: string;
            SAVE_STATE_SUCCESS: string;
            REHYDRATE_STATE_FAILED: string;
            REHYDRATE_STATE_SUCCESS: string;
            INSTANT_LOCK: string;
            TX_METADATA: string;
            HEADERS_SYNC_PROGRESS: string;
            TRANSACTIONS_SYNC_PROGRESS: string;
        };
        plugins: {
            StandardPlugin: typeof import("@dashevo/wallet-libsrc/plugins/StandardPlugin");
            Worker: typeof import("@dashevo/wallet-libsrc/plugins/Worker");
        };
        utils: {
            extendTransactionsWithMetadata: (transactions: any, transactionsMetadata: any) => any[];
            varIntSizeBytesFromLength: (length: any) => number;
            calculateTransactionFees: typeof import("@dashevo/wallet-libsrc/utils/calculateTransactionFees");
            categorizeTransactions: typeof import("@dashevo/wallet-libsrc/utils/categorizeTransactions");
            mnemonicToHDPrivateKey: (mnemonic: any, network?: any, passphrase?: string) => any;
            calculateDuffBalance: (addresses: any, chainStore: any, type?: {
                confirmed: any;
                unconfirmed: any;
                total: any;
            }) => number;
            generateNewMnemonic: () => any;
            seedToHDPrivateKey: (seed: any, network?: string) => any;
            mnemonicToWalletId: (mnemonic: any) => string;
            filterTransactions: typeof import("@dashevo/wallet-libsrc/utils/filterTransactions");
            classifyAddresses: typeof import("@dashevo/wallet-libsrc/utils/classifyAddresses");
            mnemonicToSeed: (mnemonic: any, password?: string) => any;
            feeCalculation: (type?: string) => {
                type: null;
                value: null;
            };
            coinSelection: (utxosList: any, outputsList: any, deductFee?: boolean, feeCategory?: string, strategy?: (utxosList: any, outputsList: any, deductFee?: any, feeCategory?: any) => {
                utxos: never[];
                outputs: never[];
                feeCategory: any;
                estimatedFee: number;
                utxosValue: number;
            }) => {
                utxos: never[];
                outputs: never[];
                feeCategory: any;
                estimatedFee: number;
                utxosValue: number;
            };
            doubleSha256: (data: any) => Buffer;
            dashToDuffs: typeof import("@dashevo/wallet-libsrc/utils/dashToDuffs");
            duffsToDash: typeof import("@dashevo/wallet-libsrc/utils/duffsToDash");
            fundWallet: typeof import("@dashevo/wallet-libsrc/utils/fundWallet");
            getBytesOf: typeof import("@dashevo/wallet-libsrc/utils/getBytesOf");
            hasMethod: typeof import("@dashevo/wallet-libsrc/utils/hasMethod");
            hasProp: typeof import("@dashevo/wallet-libsrc/utils/hasProp");
            sha256: (data: any) => Buffer;
            hash: (alg: any, data: any) => Buffer;
            is: {
                arr: (arr: any) => boolean;
                num: (num: any) => boolean;
                float: (float: any) => boolean;
                int: (int: any) => boolean;
                hex: (h: any) => boolean;
                string: (str: any) => boolean;
                bool: (b: any) => boolean;
                obj: (obj: any) => boolean;
                fn: (fn: any) => boolean;
                type(val: any, type: any): boolean;
                def: (val: any) => boolean;
                undef: (val: any) => boolean;
                null: (val: any) => boolean;
                exist: (val: any) => boolean;
                undefOrNull: (val: any) => boolean;
                promise: (fn: any) => boolean;
                JSON(val: any): boolean;
                stringified(val: any): boolean;
                mnemonic: (mnemonic: any) => boolean;
                network: (network: any) => boolean;
                publicKey: (pKey: any) => any;
                privateKey: (pKey: any) => any;
                HDPrivateKey: (hdKey: any) => any;
                HDPublicKey: (hdKey: any) => any;
                seed: (seed: any) => boolean;
                address: (addr: any) => boolean;
                addressObj: (addrObj: any) => boolean;
                transactionObj: (tx: any) => boolean;
                dashcoreTransaction: (tx: any) => boolean;
                feeRate: (feeRate: any) => boolean;
                txid: (txid: any) => boolean;
                utxo: (utxo: any) => boolean;
                output: (output: any) => boolean;
                rawtx: (rawtx: any) => boolean;
            };
        };
    };
    const PlatformProtocol: any;
    const Essentials: {
        Buffer: typeof Buffer;
    };
    const Errors: {
        StateTransitionBroadcastError: typeof StateTransitionBroadcastError;
    };
}
export { SDK as default };
