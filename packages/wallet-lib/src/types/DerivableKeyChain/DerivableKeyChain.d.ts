import {PrivateKey, Network,} from "../types";
import {HDPrivateKey, HDPublicKey} from "@dashevo/dashcore-lib";
import {Transaction} from "@dashevo/dashcore-lib/typings/transaction/Transaction";

export declare namespace DerivableKeyChain {
    interface IDerivableKeyChainOptions {
        network?: Network;
        keys?: [Keys]
    }
}

type keyChainId = string;
type rootKey = any;
type firstUnusedAddress = {
  path: string;
  address: string
}


export declare class DerivableKeyChain {
    constructor(options?: DerivableKeyChain.IDerivableKeyChainOptions);
    network: Network;
    keys: [Keys];

    type: HDKeyTypesParam|PrivateKeyTypeParam;
    HDPrivateKey?: HDPrivateKey;
    privateKey?: PrivateKey;

    getForPath(path: string, opts: any): any;
    getForAddress(address): any;

    getDIP15ExtendedKey(userUniqueId: string, contactUniqueId: string, index?: number, accountIndex?: number, type?: HDKeyTypesParam): HDKeyTypes;
    getFirstUnusedAddress(): firstUnusedAddress;
    getHardenedBIP44HDKey(type?: HDKeyTypesParam): HDKeyTypes;
    getHardenedDIP9FeatureHDKey(type?: HDKeyTypesParam): HDKeyTypes;
    getHardenedDIP15AccountKey(index?: number, type?: HDKeyTypesParam): HDKeyTypes;
    getRootKey(): rootKey;
    getWatchedAddresses(): Array<any>;
    getIssuedPaths(): Array<any>;
    maybeLookAhead(): any;
    markAddressAsUsed(address: string): any;
    sign(object: Transaction|any, privateKeys:[PrivateKey], sigType: number): any;
}

type HDKeyTypes = HDPublicKey | HDPrivateKey;

export declare enum HDKeyTypesParam {
    HDPrivateKey="HDPrivateKey",
    HDPublicKey="HDPrivateKey",
}
export declare enum PrivateKeyTypeParam {
    privateKey='privateKey'
}
export declare interface Keys {
    [path: string]: {
        path: string
    };
}


